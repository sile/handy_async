use futures::{self, Poll, Async, Future, BoxFuture};

use pattern::{Pattern, Branch, Iter};
use pattern::combinators::{Map, AndThen, Then, OrElse, Or, Chain, IterFold};

pub trait Matcher {
    type Error;
}

pub trait AsyncMatch<M>: Pattern
    where M: Matcher
{
    type Future: Future<Item = (M, Self::Value), Error = (M, M::Error)>;
    fn async_match(self, matcher: M) -> Self::Future;
    fn boxed(self) -> BoxPattern<M, Self::Value>
        where Self: 'static,
              Self::Future: Send + 'static
    {
        let mut f = Some(move |matcher: M| self.async_match(matcher).boxed());
        BoxPattern(Box::new(move |matcher| (f.take().unwrap())(matcher)))
    }
}

pub struct BoxPattern<M: Matcher, T>(Box<FnMut(M) -> BoxFuture<(M, T), (M, M::Error)>>);
impl<M: Matcher, T> Pattern for BoxPattern<M, T> {
    type Value = T;
}
impl<M: Matcher, T> AsyncMatch<M> for BoxPattern<M, T> {
    type Future = BoxFuture<(M, T), (M, M::Error)>;
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
impl<M: Matcher, P, F, T> AsyncMatch<M> for Map<P, F>
    where F: FnOnce(P::Value) -> T,
          P: AsyncMatch<M>
{
    type Future = MatchMap<<P as AsyncMatch<M>>::Future, F>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p, f) = self.unwrap();
        MatchMap(Some((p.async_match(matcher), f)))
    }
}

pub struct MatchAndThen<M, P0, P1, F>(Phase<(P0::Future, F), P1::Future>)
    where M: Matcher,
          P0: AsyncMatch<M>,
          P1: AsyncMatch<M>,
          F: FnOnce(P0::Value) -> P1;
impl<M: Matcher, P0, P1, F> Future for MatchAndThen<M, P0, P1, F>
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M>,
          F: FnOnce(P0::Value) -> P1
{
    type Item = (M, P1::Value);
    type Error = (M, M::Error);
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
impl<M: Matcher, P0, P1, F> AsyncMatch<M> for AndThen<P0, F>
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M>,
          F: FnOnce(P0::Value) -> P1
{
    type Future = MatchAndThen<M, P0, P1, F>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p, f) = self.unwrap();
        MatchAndThen(Phase::A((p.async_match(matcher), f)))
    }
}

pub struct MatchThen<M: Matcher, P0, P1, F>(Phase<(P0::Future, F), P1::Future>)
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M>,
          F: FnOnce(Result<P0::Value, M::Error>) -> P1;
impl<M: Matcher, P0, P1, F> Future for MatchThen<M, P0, P1, F>
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M>,
          F: FnOnce(Result<P0::Value, M::Error>) -> P1
{
    type Item = (M, P1::Value);
    type Error = (M, M::Error);
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
impl<M: Matcher, P0, P1, F> AsyncMatch<M> for Then<P0, F, M::Error>
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M>,
          F: FnOnce(Result<P0::Value, M::Error>) -> P1
{
    type Future = MatchThen<M, P0, P1, F>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p, f) = self.unwrap();
        MatchThen(Phase::A((p.async_match(matcher), f)))
    }
}

pub struct MatchOrElse<M: Matcher, P0, P1, F>(Phase<(P0::Future, F), P1::Future>)
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M>,
          F: FnOnce(M::Error) -> P1;
impl<M: Matcher, P0, P1, F> Future for MatchOrElse<M, P0, P1, F>
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M, Value = P0::Value>,
          F: FnOnce(M::Error) -> P1
{
    type Item = (M, P1::Value);
    type Error = (M, M::Error);
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
impl<M: Matcher, P0, P1, F> AsyncMatch<M> for OrElse<P0, F, M::Error>
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M, Value = P0::Value>,
          F: FnOnce(M::Error) -> P1
{
    type Future = MatchOrElse<M, P0, P1, F>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p, f) = self.unwrap();
        MatchOrElse(Phase::A((p.async_match(matcher), f)))
    }
}

pub struct MatchOr<M: Matcher, P0, P1>(Phase<(P0::Future, P1), P1::Future>)
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M>;
impl<M: Matcher, P0, P1> Future for MatchOr<M, P0, P1>
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M, Value = P0::Value>
{
    type Item = (M, P1::Value);
    type Error = (M, M::Error);
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
impl<M: Matcher, P0, P1> AsyncMatch<M> for Or<P0, P1>
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M, Value = P0::Value>
{
    type Future = MatchOr<M, P0, P1>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p0, p1) = self.unwrap();
        MatchOr(Phase::A((p0.async_match(matcher), p1)))
    }
}

pub struct MatchChain<M: Matcher, P0, P1>(Phase<(P0::Future, P1), (P1::Future, P0::Value)>)
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M>;
impl<M: Matcher, P0, P1> Future for MatchChain<M, P0, P1>
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M>
{
    type Item = (M, (P0::Value, P1::Value));
    type Error = (M, M::Error);
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
impl<M: Matcher, P0, P1> AsyncMatch<M> for Chain<P0, P1>
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M>
{
    type Future = MatchChain<M, P0, P1>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p0, p1) = self.unwrap();
        MatchChain(Phase::A((p0.async_match(matcher), p1)))
    }
}

pub struct MatchOption<M: Matcher, P>(Option<Result<P::Future, M>>) where P: AsyncMatch<M>;
impl<M: Matcher, P> Future for MatchOption<M, P>
    where P: AsyncMatch<M>
{
    type Item = (M, Option<P::Value>);
    type Error = (M, M::Error);
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
impl<M: Matcher, P> AsyncMatch<M> for Option<P>
    where P: AsyncMatch<M>
{
    type Future = MatchOption<M, P>;
    fn async_match(self, matcher: M) -> Self::Future {
        if let Some(p) = self {
            MatchOption(Some(Ok(p.async_match(matcher))))
        } else {
            MatchOption(Some(Err(matcher)))
        }
    }
}

impl<M: Matcher, T> AsyncMatch<M> for Result<T, M::Error> {
    type Future = futures::Done<(M, T), (M, M::Error)>;
    fn async_match(self, matcher: M) -> Self::Future {
        match self {
            Ok(v) => futures::done(Ok((matcher, v))),
            Err(e) => futures::done(Err((matcher, e))),
        }
    }
}

pub type MatchBranch<M, A, B, C, D, E, F, G, H>
    where A: AsyncMatch<M>,
          B: AsyncMatch<M, Value = A::Value>,
          C: AsyncMatch<M, Value = A::Value>,
          D: AsyncMatch<M, Value = A::Value>,
          E: AsyncMatch<M, Value = A::Value>,
          F: AsyncMatch<M, Value = A::Value>,
          G: AsyncMatch<M, Value = A::Value>,
          H: AsyncMatch<M, Value = A::Value> = Branch<A::Future,
                                                      B::Future,
                                                      C::Future,
                                                      D::Future,
                                                      E::Future,
                                                      F::Future,
                                                      G::Future,
                                                      H::Future>;
impl<M, A, B, C, D, E, F, G, H> AsyncMatch<M> for Branch<A, B, C, D, E, F, G, H>
    where M: Matcher,
          A: AsyncMatch<M>,
          B: AsyncMatch<M, Value = A::Value>,
          C: AsyncMatch<M, Value = A::Value>,
          D: AsyncMatch<M, Value = A::Value>,
          E: AsyncMatch<M, Value = A::Value>,
          F: AsyncMatch<M, Value = A::Value>,
          G: AsyncMatch<M, Value = A::Value>,
          H: AsyncMatch<M, Value = A::Value>
{
    type Future = MatchBranch<M, A, B, C, D, E, F, G, H>;
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

pub struct MatchIterFold<M:Matcher, I, F, T>
    (Phase<(<I::Item as AsyncMatch<M>>::Future, I, T, F), (M, T)>)
    where I: Iterator,
          I::Item: AsyncMatch<M>;
impl<M: Matcher, I, F, T> Future for MatchIterFold<M, I, F, T>
    where I: Iterator,
          I::Item: AsyncMatch<M>,
          F: Fn(T, <I::Item as Pattern>::Value) -> T
{
    type Item = (M, T);
    type Error = (M, M::Error);
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
impl<M: Matcher, I, F, T> AsyncMatch<M> for IterFold<I, F, T>
    where I: Iterator,
          I::Item: AsyncMatch<M>,
          F: Fn(T, <I::Item as Pattern>::Value) -> T
{
    type Future = MatchIterFold<M, I, F, T>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (mut iter, fold, acc) = self.unwrap();
        if let Some(p) = iter.next() {
            MatchIterFold(Phase::A((p.async_match(matcher), iter, acc, fold)))
        } else {
            MatchIterFold(Phase::B((matcher, acc)))
        }
    }
}

pub struct MatchIter<M:Matcher, I>(Phase<(<I::Item as AsyncMatch<M>>::Future, I), M>)
    where I: Iterator,
          I::Item: AsyncMatch<M>;
impl<M: Matcher, I> Future for MatchIter<M, I>
    where I: Iterator,
          I::Item: AsyncMatch<M>
{
    type Item = (M, ());
    type Error = (M, M::Error);
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
impl<M: Matcher, I> AsyncMatch<M> for Iter<I>
    where I: Iterator,
          I::Item: AsyncMatch<M>
{
    type Future = MatchIter<M, I>;
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
