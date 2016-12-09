use futures::{self, Poll, Async, Future, BoxFuture};

use pattern::{Pattern, Branch, Iter};
use pattern::combinators::{Map, AndThen, Then, OrElse, Or, Chain, IterFold};

pub trait AsyncMatch<M, E>: Pattern {
    type Future: Future<Item = (M, Self::Value), Error = (M, E)>;
    fn async_match(self, matcher: M) -> Self::Future;
    fn boxed(self) -> BoxPattern<M, Self::Value, E>
        where Self: 'static,
              Self::Future: Send + 'static
    {
        let mut f = Some(move |matcher: M| self.async_match(matcher).boxed());
        BoxPattern(Box::new(move |matcher| (f.take().unwrap())(matcher)))
    }
}

// pub fn lossy();
// pub fn sync();

pub struct BoxPattern<M, T, E>(Box<FnMut(M) -> BoxFuture<(M, T), (M, E)>>);
impl<M, T, E> Pattern for BoxPattern<M, T, E> {
    type Value = T;
}
impl<M, T, E> AsyncMatch<M, E> for BoxPattern<M, T, E> {
    type Future = BoxFuture<(M, T), (M, E)>;
    fn async_match(mut self, matcher: M) -> Self::Future {
        (self.0)(matcher)
    }
}

pub struct MatchMap<P, F>(Option<(P, F)>);
impl<M, P, F, T, U> Future for MatchMap<P, F>
    where P: Future<Item = (M, T)>,
          F: FnOnce(T) -> U
{
    type Item = (M, U);
    type Error = P::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (mut p, f) = self.0.take().expect("Cannot poll MatchMap twice");
        if let Async::Ready((matcher, v)) = p.poll()? {
            Ok(Async::Ready((matcher, f(v))))
        } else {
            self.0 = Some((p, f));
            Ok(Async::NotReady)
        }
    }
}
impl<M, P: Pattern, E, F, T> AsyncMatch<M, E> for Map<P, F>
    where F: FnOnce(P::Value) -> T,
          P: AsyncMatch<M, E>
{
    type Future = MatchMap<<P as AsyncMatch<M, E>>::Future, F>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p, f) = self.unwrap();
        MatchMap(Some((p.async_match(matcher), f)))
    }
}

pub struct MatchAndThen<M, E, P0, P1, F>(Phase<(P0::Future, F), P1::Future>)
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E>,
          F: FnOnce(P0::Value) -> P1;
impl<M, E, P0, P1, F> Future for MatchAndThen<M, E, P0, P1, F>
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E>,
          F: FnOnce(P0::Value) -> P1
{
    type Item = (M, P1::Value);
    type Error = (M, E);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut p0, f)) => {
                if let Async::Ready((m, v0)) = p0.poll()? {
                    let p1 = f(v0).async_match(m);
                    self.0 = Phase::B(p1);
                    self.poll()
                } else {
                    self.0 = Phase::A((p0, f));
                    Ok(Async::NotReady)
                }
            }
            Phase::B(mut p1) => {
                if let Async::Ready((m, v1)) = p1.poll()? {
                    Ok(Async::Ready((m, v1)))
                } else {
                    self.0 = Phase::B(p1);
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchAndThen twice"),
        }
    }
}
impl<M, E, P0, P1, F> AsyncMatch<M, E> for AndThen<P0, F>
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E>,
          F: FnOnce(P0::Value) -> P1
{
    type Future = MatchAndThen<M, E, P0, P1, F>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p, f) = self.unwrap();
        MatchAndThen(Phase::A((p.async_match(matcher), f)))
    }
}

pub struct MatchThen<M, E, P0, P1, F>(Phase<(P0::Future, F), P1::Future>)
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E>,
          F: FnOnce(Result<P0::Value, E>) -> P1;
impl<M, E, P0, P1, F> Future for MatchThen<M, E, P0, P1, F>
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E>,
          F: FnOnce(Result<P0::Value, E>) -> P1
{
    type Item = (M, P1::Value);
    type Error = (M, E);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut p0, f)) => {
                match p0.poll() {
                    Err((m, e)) => {
                        let p1 = f(Err(e)).async_match(m);
                        self.0 = Phase::B(p1);
                        self.poll()
                    }
                    Ok(Async::Ready((m, v0))) => {
                        let p1 = f(Ok(v0)).async_match(m);
                        self.0 = Phase::B(p1);
                        self.poll()
                    }
                    Ok(Async::NotReady) => {
                        self.0 = Phase::A((p0, f));
                        Ok(Async::NotReady)
                    }
                }
            }
            Phase::B(mut p1) => {
                if let Async::Ready((m, v1)) = p1.poll()? {
                    Ok(Async::Ready((m, v1)))
                } else {
                    self.0 = Phase::B(p1);
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchThen twice"),
        }
    }
}
impl<M, E, P0, P1, F> AsyncMatch<M, E> for Then<P0, F, E>
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E>,
          F: FnOnce(Result<P0::Value, E>) -> P1
{
    type Future = MatchThen<M, E, P0, P1, F>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p, f) = self.unwrap();
        MatchThen(Phase::A((p.async_match(matcher), f)))
    }
}

pub struct MatchOrElse<M, E, P0, P1, F>(Phase<(P0::Future, F), P1::Future>)
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E>,
          F: FnOnce(E) -> P1;
impl<M, E, P0, P1, F> Future for MatchOrElse<M, E, P0, P1, F>
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E, Value = P0::Value>,
          F: FnOnce(E) -> P1
{
    type Item = (M, P1::Value);
    type Error = (M, E);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut p0, f)) => {
                match p0.poll() {
                    Err((m, e)) => {
                        let p1 = f(e).async_match(m);
                        self.0 = Phase::B(p1);
                        self.poll()
                    }
                    Ok(Async::Ready((m, v0))) => Ok(Async::Ready((m, v0))),
                    Ok(Async::NotReady) => {
                        self.0 = Phase::A((p0, f));
                        Ok(Async::NotReady)
                    }
                }
            }
            Phase::B(mut p1) => {
                if let Async::Ready((m, v1)) = p1.poll()? {
                    Ok(Async::Ready((m, v1)))
                } else {
                    self.0 = Phase::B(p1);
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchOrElse twice"),
        }
    }
}
impl<M, E, P0, P1, F> AsyncMatch<M, E> for OrElse<P0, F, E>
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E, Value = P0::Value>,
          F: FnOnce(E) -> P1
{
    type Future = MatchOrElse<M, E, P0, P1, F>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p, f) = self.unwrap();
        MatchOrElse(Phase::A((p.async_match(matcher), f)))
    }
}

pub struct MatchOr<M, E, P0, P1>(Phase<(P0::Future, P1), P1::Future>)
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E>;
impl<M, E, P0, P1> Future for MatchOr<M, E, P0, P1>
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E, Value = P0::Value>
{
    type Item = (M, P1::Value);
    type Error = (M, E);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut p0, p1)) => {
                match p0.poll() {
                    Err((m, _)) => {
                        let p1 = p1.async_match(m);
                        self.0 = Phase::B(p1);
                        self.poll()
                    }
                    Ok(Async::Ready((m, v0))) => Ok(Async::Ready((m, v0))),
                    Ok(Async::NotReady) => {
                        self.0 = Phase::A((p0, p1));
                        Ok(Async::NotReady)
                    }
                }
            }
            Phase::B(mut p1) => {
                if let Async::Ready((m, v1)) = p1.poll()? {
                    Ok(Async::Ready((m, v1)))
                } else {
                    self.0 = Phase::B(p1);
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchOr twice"),
        }
    }
}
impl<M, E, P0, P1> AsyncMatch<M, E> for Or<P0, P1>
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E, Value = P0::Value>
{
    type Future = MatchOr<M, E, P0, P1>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p0, p1) = self.unwrap();
        MatchOr(Phase::A((p0.async_match(matcher), p1)))
    }
}

pub struct MatchChain<M, E, P0, P1>(Phase<(P0::Future, P1), (P1::Future, P0::Value)>)
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E>;
impl<M, E, P0, P1> Future for MatchChain<M, E, P0, P1>
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E>
{
    type Item = (M, (P0::Value, P1::Value));
    type Error = (M, E);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut p0, p1)) => {
                match p0.poll() {
                    Err((m, e)) => Err((m, e)),
                    Ok(Async::Ready((m, v0))) => {
                        self.0 = Phase::B((p1.async_match(m), v0));
                        self.poll()
                    }
                    Ok(Async::NotReady) => {
                        self.0 = Phase::A((p0, p1));
                        Ok(Async::NotReady)
                    }
                }
            }
            Phase::B((mut p1, v0)) => {
                if let Async::Ready((m, v1)) = p1.poll()? {
                    Ok(Async::Ready((m, (v0, v1))))
                } else {
                    self.0 = Phase::B((p1, v0));
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchChain twice"),
        }
    }
}
impl<M, E, P0, P1> AsyncMatch<M, E> for Chain<P0, P1>
    where P0: AsyncMatch<M, E>,
          P1: AsyncMatch<M, E>
{
    type Future = MatchChain<M, E, P0, P1>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p0, p1) = self.unwrap();
        MatchChain(Phase::A((p0.async_match(matcher), p1)))
    }
}

pub struct MatchOption<M, E, P>(Option<Result<P::Future, M>>) where P: AsyncMatch<M, E>;
impl<M, E, P> Future for MatchOption<M, E, P>
    where P: AsyncMatch<M, E>
{
    type Item = (M, Option<P::Value>);
    type Error = (M, E);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let inner = self.0.take().expect("Cannot poll MatchOption twice");
        match inner {
            Ok(mut f) => {
                if let Async::Ready((m, v)) = f.poll()? {
                    Ok(Async::Ready((m, Some(v))))
                } else {
                    self.0 = Some(Ok(f));
                    Ok(Async::NotReady)
                }
            }
            Err(m) => Ok(Async::Ready((m, None))),
        }
    }
}
impl<M, E, P> AsyncMatch<M, E> for Option<P>
    where P: AsyncMatch<M, E>
{
    type Future = MatchOption<M, E, P>;
    fn async_match(self, matcher: M) -> Self::Future {
        if let Some(p) = self {
            MatchOption(Some(Ok(p.async_match(matcher))))
        } else {
            MatchOption(Some(Err(matcher)))
        }
    }
}

impl<M, E, T> AsyncMatch<M, E> for Result<T, E> {
    type Future = futures::Done<(M, T), (M, E)>;
    fn async_match(self, matcher: M) -> Self::Future {
        match self {
            Ok(v) => futures::done(Ok((matcher, v))),
            Err(e) => futures::done(Err((matcher, e))),
        }
    }
}

pub type MatchBranch<M, ER, A, B, C, D, E, F, G, H>
    where A: AsyncMatch<M, ER>,
          B: AsyncMatch<M, ER, Value = A::Value>,
          C: AsyncMatch<M, ER, Value = A::Value>,
          D: AsyncMatch<M, ER, Value = A::Value>,
          E: AsyncMatch<M, ER, Value = A::Value>,
          F: AsyncMatch<M, ER, Value = A::Value>,
          G: AsyncMatch<M, ER, Value = A::Value>,
          H: AsyncMatch<M, ER, Value = A::Value> = Branch<A::Future,
                                                          B::Future,
                                                          C::Future,
                                                          D::Future,
                                                          E::Future,
                                                          F::Future,
                                                          G::Future,
                                                          H::Future>;
impl<M, ER, A, B, C, D, E, F, G, H> AsyncMatch<M, ER> for Branch<A, B, C, D, E, F, G, H>
    where A: AsyncMatch<M, ER>,
          B: AsyncMatch<M, ER, Value = A::Value>,
          C: AsyncMatch<M, ER, Value = A::Value>,
          D: AsyncMatch<M, ER, Value = A::Value>,
          E: AsyncMatch<M, ER, Value = A::Value>,
          F: AsyncMatch<M, ER, Value = A::Value>,
          G: AsyncMatch<M, ER, Value = A::Value>,
          H: AsyncMatch<M, ER, Value = A::Value>
{
    type Future = MatchBranch<M, ER, A, B, C, D, E, F, G, H>;
    fn async_match(self, matcher: M) -> Self::Future {
        match self {
            Branch::A(p) => Branch::A(p.async_match(matcher)),
            Branch::B(p) => Branch::B(p.async_match(matcher)),
            Branch::C(p) => Branch::C(p.async_match(matcher)),
            Branch::D(p) => Branch::D(p.async_match(matcher)),
            Branch::E(p) => Branch::E(p.async_match(matcher)),
            Branch::F(p) => Branch::F(p.async_match(matcher)),
            Branch::G(p) => Branch::G(p.async_match(matcher)),
            Branch::H(p) => Branch::H(p.async_match(matcher)),
        }
    }
}

pub struct MatchIterFold<M, E, I, F, T>
    (Phase<(<I::Item as AsyncMatch<M, E>>::Future, I, T, F), (M, T)>)
    where I: Iterator,
          I::Item: AsyncMatch<M, E>;
impl<M, E, I, F, T> Future for MatchIterFold<M, E, I, F, T>
    where I: Iterator,
          I::Item: AsyncMatch<M, E>,
          F: Fn(T, <I::Item as Pattern>::Value) -> T
{
    type Item = (M, T);
    type Error = (M, E);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut f, mut iter, acc, fold)) => {
                if let Async::Ready((m, v)) = f.poll()? {
                    let acc = fold(acc, v);
                    if let Some(p) = iter.next() {
                        self.0 = Phase::A((p.async_match(m), iter, acc, fold));
                        self.poll()
                    } else {
                        Ok(Async::Ready((m, acc)))
                    }
                } else {
                    self.0 = Phase::A((f, iter, acc, fold));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((m, v)) => Ok(Async::Ready((m, v))),
            _ => panic!("Cannot poll MatchIterFold twice"),
        }
    }
}
impl<M, E, I, F, T> AsyncMatch<M, E> for IterFold<I, F, T>
    where I: Iterator,
          I::Item: AsyncMatch<M, E>,
          F: Fn(T, <I::Item as Pattern>::Value) -> T
{
    type Future = MatchIterFold<M, E, I, F, T>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (mut iter, fold, acc) = self.unwrap();
        if let Some(p) = iter.next() {
            MatchIterFold(Phase::A((p.async_match(matcher), iter, acc, fold)))
        } else {
            MatchIterFold(Phase::B((matcher, acc)))
        }
    }
}

pub struct MatchIter<M, E, I>(Phase<(<I::Item as AsyncMatch<M,E>>::Future, I), M>)
    where I: Iterator,
          I::Item: AsyncMatch<M, E>;
impl<M, E, I> Future for MatchIter<M, E, I>
    where I: Iterator,
          I::Item: AsyncMatch<M, E>
{
    type Item = (M, ());
    type Error = (M, E);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut f, mut iter)) => {
                if let Async::Ready((m, _)) = f.poll()? {
                    if let Some(p) = iter.next() {
                        self.0 = Phase::A((p.async_match(m), iter));
                        self.poll()
                    } else {
                        Ok(Async::Ready((m, ())))
                    }
                } else {
                    self.0 = Phase::A((f, iter));
                    Ok(Async::NotReady)
                }
            }
            Phase::B(m) => Ok(Async::Ready((m, ()))),
            _ => panic!("Cannot poll MatchIter twice"),
        }
    }
}
impl<M, E, I> AsyncMatch<M, E> for Iter<I>
    where I: Iterator,
          I::Item: AsyncMatch<M, E>
{
    type Future = MatchIter<M, E, I>;
    fn async_match(self, matcher: M) -> Self::Future {
        let mut iter = self.0;
        if let Some(p) = iter.next() {
            MatchIter(Phase::A((p.async_match(matcher), iter)))
        } else {
            MatchIter(Phase::B(matcher))
        }
    }
}

#[derive(Debug)]
enum Phase<A, B> {
    A(A),
    B(B),
    Polled,
}
impl<A, B> Phase<A, B> {
    pub fn take(&mut self) -> Self {
        use std::mem;
        mem::replace(self, Phase::Polled)
    }
}
