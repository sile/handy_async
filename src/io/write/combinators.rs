use std::io::{self, Write};
use futures::{self, Poll, Async, Future};

use pattern::{self, Pattern, Branch};
use super::WriteTo;
use super::super::common::{self, Phase};

/// A future for writing `Then` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::WriteTo;
/// use handy_io::pattern::Pattern;
///
/// let then_pattern = 1u8.then(|r| if r.is_ok() { Ok(true) } else { Ok(false) });
/// let mut buf = [0; 1];
/// let value = then_pattern.sync_write_to(&mut &mut buf[..]).unwrap();
/// assert_eq!(value, true);
/// assert_eq!(buf, [1]);
/// ```
pub struct WriteThen<W: Write, P0, P1, F>(Phase<(P0::Future, F), P1::Future>)
    where P0: WriteTo<W>,
          P1: WriteTo<W>;
impl<W: Write, P0, P1, F> Future for WriteThen<W, P0, P1, F>
    where P0: WriteTo<W>,
          P1: WriteTo<W>,
          F: FnOnce(io::Result<P0::Value>) -> P1
{
    type Item = (W, P1::Value);
    type Error = (W, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut f, then)) => {
                match f.poll() {
                    Err((w, e)) => {
                        self.0 = Phase::B(then(Err(e)).lossless_write_to(w));
                        self.poll()
                    }
                    Ok(Async::Ready((w, v0))) => {
                        self.0 = Phase::B(then(Ok(v0)).lossless_write_to(w));
                        self.poll()
                    }
                    Ok(Async::NotReady) => {
                        self.0 = Phase::A((f, then));
                        Ok(Async::NotReady)
                    }
                }
            }
            Phase::B(mut f) => {
                let result = f.poll()?;
                if let Async::NotReady = result {
                    self.0 = Phase::B(f);
                }
                Ok(result)
            }
            _ => panic!("Cannot poll WriteThen twice"),
        }
    }
}
impl<W: Write, P0, P1, F> WriteTo<W> for pattern::combinators::Then<P0, F>
    where P0: WriteTo<W>,
          P1: WriteTo<W>,
          F: FnOnce(io::Result<P0::Value>) -> P1
{
    type Future = WriteThen<W, P0, P1, F>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        let (p0, then) = self.unwrap();
        WriteThen(Phase::A((p0.lossless_write_to(writer), then)))
    }
}

/// A future for writing `AndThen` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::WriteTo;
/// use handy_io::pattern::Pattern;
///
/// let and_then_pattern = 0u8.and_then(|_| 1u8);
/// let mut buf = [0; 2];
/// and_then_pattern.sync_write_to(&mut &mut buf[..]).unwrap();
/// assert_eq!(buf, [0, 1]);
/// ```
pub struct WriteAndThen<W, P0, P1, F>(WriteAndThenInner<W, P0, P1, F>)
    where P0: WriteTo<W>,
          P1: WriteTo<W>,
          F: FnOnce(P0::Value) -> P1,
          W: Write;
type WriteAndThenInner<W, P0, P1, F>
    where P0: WriteTo<W>,
          P1: WriteTo<W>,
          F: FnOnce(P0::Value) -> P1,
          W: Write = WriteThen<W,
                               (P0, io::Result<F>),
                               Branch<P1, io::Result<P1::Value>>,
                               fn(io::Result<(P0::Value, F)>) -> Branch<P1, io::Result<P1::Value>>>;
impl<W: Write, P0, P1, F> Future for WriteAndThen<W, P0, P1, F>
    where P0: WriteTo<W>,
          P1: WriteTo<W>,
          F: FnOnce(P0::Value) -> P1
{
    type Item = (W, P1::Value);
    type Error = (W, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}
impl<W: Write, P0, P1, F> WriteTo<W> for pattern::combinators::AndThen<P0, F>
    where P0: WriteTo<W>,
          P1: WriteTo<W>,
          F: FnOnce(P0::Value) -> P1
{
    type Future = WriteAndThen<W, P0, P1, F>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        let (p0, and_then) = self.unwrap();
        WriteAndThen((p0, Ok(and_then))
            .then(common::then_to_and_then as _)
            .lossless_write_to(writer))
    }
}

/// A future for writing `OrElse` pattern.
///
/// # Example
///
/// ```
/// use std::io::{Error, ErrorKind};
/// use handy_io::io::WriteTo;
/// use handy_io::pattern::Pattern;
///
/// let or_else_pattern = Err(Error::new(ErrorKind::Other, "")).or_else(|_| Ok(0xFF));
/// let mut buf = [];
/// let value = or_else_pattern.sync_write_to(&mut &mut buf[..]).unwrap();
/// assert_eq!(value, 0xFF);
/// ```
pub struct WriteOrElse<W: Write, P0, P1, F>(Phase<(P0::Future, F), P1::Future>)
    where P0: WriteTo<W>,
          P1: WriteTo<W>;
impl<W: Write, P0, P1, F> Future for WriteOrElse<W, P0, P1, F>
    where P0: WriteTo<W>,
          P1: WriteTo<W, Value = P0::Value>,
          F: FnOnce(io::Error) -> P1
{
    type Item = (W, P0::Value);
    type Error = (W, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut f, or_else)) => {
                match f.poll() {
                    Err((w, e)) => {
                        self.0 = Phase::B(or_else(e).lossless_write_to(w));
                        self.poll()
                    }
                    Ok(result) => {
                        if let Async::NotReady = result {
                            self.0 = Phase::A((f, or_else));
                        }
                        Ok(result)
                    }
                }
            }
            Phase::B(mut f) => {
                let result = f.poll()?;
                if let Async::NotReady = result {
                    self.0 = Phase::B(f);
                }
                Ok(result)
            }
            _ => panic!("Cannot poll WriteOrElse twice"),
        }
    }
}
impl<W: Write, P0, P1, F> WriteTo<W> for pattern::combinators::OrElse<P0, F>
    where P0: WriteTo<W>,
          P1: WriteTo<W, Value = P0::Value>,
          F: FnOnce(io::Error) -> P1
{
    type Future = WriteOrElse<W, P0, P1, F>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        let (p0, or_else) = self.unwrap();
        WriteOrElse(Phase::A((p0.lossless_write_to(writer), or_else)))
    }
}

/// A future for writing `Map` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::WriteTo;
/// use handy_io::pattern::Pattern;
///
/// let map_pattern = 0u8.map(|_| 1);
/// let mut buf = [1];
/// let value = map_pattern.sync_write_to(&mut &mut buf[..]).unwrap();
/// assert_eq!(value, 1);
/// ```
pub struct WriteMap<W: Write, P, F>(Option<(P::Future, F)>) where P: WriteTo<W>;
impl<W: Write, P, F, T> Future for WriteMap<W, P, F>
    where P: WriteTo<W>,
          F: FnOnce(P::Value) -> T
{
    type Item = (W, T);
    type Error = (W, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (mut f, map) = self.0.take().expect("Cannot poll WriteMap twice");
        if let Async::Ready((w, v)) = f.poll()? {
            Ok(Async::Ready((w, map(v))))
        } else {
            self.0 = Some((f, map));
            Ok(Async::NotReady)
        }
    }
}
impl<W: Write, P, F, T> WriteTo<W> for pattern::combinators::Map<P, F>
    where P: WriteTo<W>,
          F: FnOnce(P::Value) -> T
{
    type Future = WriteMap<W, P, F>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        let (p, f) = self.unwrap();
        WriteMap(Some((p.lossless_write_to(writer), f)))
    }
}

/// A future for writing `Chain` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::WriteTo;
/// use handy_io::pattern::Pattern;
///
/// let chain_pattern = 0u8.chain(1u8).chain(2u8);
/// let mut buf = [0; 3];
/// chain_pattern.sync_write_to(&mut &mut buf[..]).unwrap();
/// assert_eq!(buf, [0, 1, 2]);
/// ```
pub struct WriteChain<W: Write, P0, P1>(Phase<(P0::Future, P1), (P1::Future, P0::Value)>)
    where P0: WriteTo<W>,
          P1: WriteTo<W>;
impl<W: Write, P0, P1> Future for WriteChain<W, P0, P1>
    where P0: WriteTo<W>,
          P1: WriteTo<W>
{
    type Item = (W, (P0::Value, P1::Value));
    type Error = (W, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut f, p1)) => {
                if let Async::Ready((w, v0)) = f.poll()? {
                    self.0 = Phase::B((p1.lossless_write_to(w), v0));
                    self.poll()
                } else {
                    self.0 = Phase::A((f, p1));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((mut f, v0)) => {
                if let Async::Ready((w, v1)) = f.poll()? {
                    Ok(Async::Ready((w, (v0, v1))))
                } else {
                    self.0 = Phase::B((f, v0));
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll WriteChain twice"),
        }
    }
}
impl<W: Write, P0, P1> WriteTo<W> for pattern::combinators::Chain<P0, P1>
    where P0: WriteTo<W>,
          P1: WriteTo<W>
{
    type Future = WriteChain<W, P0, P1>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        let (p0, p1) = self.unwrap();
        WriteChain(Phase::A((p0.lossless_write_to(writer), p1)))
    }
}
impl<W: Write> WriteTo<W> for () {
    type Future = futures::Finished<(W, ()), (W, io::Error)>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        futures::finished((writer, self))
    }
}

impl<W: Write, P0, P1> WriteTo<W> for (P0, P1)
    where P0: WriteTo<W>,
          P1: WriteTo<W>
{
    type Future = WriteChain<W, P0, P1>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        let (p0, p1) = self;
        p0.chain(p1).lossless_write_to(writer)
    }
}

type MapFuture<W,P,T> where P: WriteTo<W> =
     <pattern::combinators::Map<P, fn (P::Value) -> T> as WriteTo<W>>::Future;
macro_rules! impl_tuple_write_from {
    ([$($p:ident),* | $pn:ident], [$($i:tt),* | $it:tt]) => {
        impl<W: Write, $($p),*, $pn> WriteTo<W> for ($($p),*, $pn)
            where $($p:WriteTo<W>,)*
                  $pn:WriteTo<W>
        {
            type Future = MapFuture<W, (($($p),*), $pn), ($($p::Value),*,$pn::Value)>;
            fn lossless_write_to(self, writer: W) -> Self::Future {
                fn flatten<$($p),*, $pn>((a, b): (($($p),*), $pn)) ->
                    ($($p),*, $pn) {
                        ($(a.$i),*, b)
                }
                (($(self.$i),*), self.$it).map(flatten as _).lossless_write_to(writer)
            }
        }
    }
}
impl_tuple_write_from!([P0, P1 | P2], [0, 1 | 2]);
impl_tuple_write_from!([P0, P1, P2 | P3], [0, 1, 2 | 3]);
impl_tuple_write_from!([P0, P1, P2, P3 | P4], [0, 1, 2, 3 | 4]);
impl_tuple_write_from!([P0, P1, P2, P3, P4 | P5], [0, 1, 2, 3, 4 | 5]);
impl_tuple_write_from!([P0, P1, P2, P3, P4, P5 | P6], [0, 1, 2, 3, 4, 5 | 6]);
impl_tuple_write_from!([P0, P1, P2, P3, P4, P5, P6 | P7], [0, 1, 2, 3, 4, 5, 6 | 7]);
impl_tuple_write_from!([P0, P1, P2, P3, P4, P5, P6, P7 | P8],
                       [0, 1, 2, 3, 4, 5, 6, 7 | 8]);
impl_tuple_write_from!([P0, P1, P2, P3, P4, P5, P6, P7, P8 | P9],
                       [0, 1, 2, 3, 4, 5, 6, 7, 8 | 9]);

/// A future for writing `Branch` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::WriteTo;
/// use handy_io::pattern::{Pattern, Branch};
///
/// let branch_pattern =
///     0u8.then(|r| if r.is_ok() { Branch::A(Ok(1)) as Branch<_, _> } else { Branch::B(Ok(2)) });
/// let mut buf = [0];
/// let value = branch_pattern.sync_write_to(&mut &mut buf[..]).unwrap();
/// assert_eq!(value, 1);
/// ```
pub type WriteBranch<W, A, B, C, D, E, F, G, H>
    where A: WriteTo<W>,
          B: WriteTo<W, Value = A::Value>,
          C: WriteTo<W, Value = A::Value>,
          D: WriteTo<W, Value = A::Value>,
          E: WriteTo<W, Value = A::Value>,
          F: WriteTo<W, Value = A::Value>,
          G: WriteTo<W, Value = A::Value>,
          H: WriteTo<W, Value = A::Value>,
          W: Write = Branch<A::Future,
                            B::Future,
                            C::Future,
                            D::Future,
                            E::Future,
                            F::Future,
                            G::Future,
                            H::Future>;
impl<W: Write, A, B, C, D, E, F, G, H> WriteTo<W> for Branch<A, B, C, D, E, F, G, H>
    where A: WriteTo<W>,
          B: WriteTo<W, Value = A::Value>,
          C: WriteTo<W, Value = A::Value>,
          D: WriteTo<W, Value = A::Value>,
          E: WriteTo<W, Value = A::Value>,
          F: WriteTo<W, Value = A::Value>,
          G: WriteTo<W, Value = A::Value>,
          H: WriteTo<W, Value = A::Value>
{
    type Future = WriteBranch<W, A, B, C, D, E, F, G, H>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        match self {
            Branch::A(p) => Branch::A(p.lossless_write_to(writer)),
            Branch::B(p) => Branch::B(p.lossless_write_to(writer)),
            Branch::C(p) => Branch::C(p.lossless_write_to(writer)),
            Branch::D(p) => Branch::D(p.lossless_write_to(writer)),
            Branch::E(p) => Branch::E(p.lossless_write_to(writer)),
            Branch::F(p) => Branch::F(p.lossless_write_to(writer)),
            Branch::G(p) => Branch::G(p.lossless_write_to(writer)),
            Branch::H(p) => Branch::H(p.lossless_write_to(writer)),
        }
    }
}

/// A future for writing `IterFold` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::WriteTo;
/// use handy_io::pattern::{Pattern, Iter};
///
/// let iter_fold_pattern = Iter(vec![0u8, 1u8].into_iter()).fold(0, |acc, _| acc + 1);
/// let mut buf = [0; 2];
/// let value = iter_fold_pattern.sync_write_to(&mut &mut buf[..]).unwrap();
/// assert_eq!(value, 2);
/// ```
pub struct WriteIterFold<W: Write, P, I, F, T>(Phase<(P::Future, I, T, F), (W, T)>)
    where P: WriteTo<W>;
impl<W: Write, I, F, T> Future for WriteIterFold<W, I::Item, I, F, T>
    where I: Iterator,
          I::Item: WriteTo<W>,
          F: Fn(T, <I::Item as Pattern>::Value) -> T
{
    type Item = (W, T);
    type Error = (W, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut f, mut iter, acc, fold)) => {
                if let Async::Ready((w, v)) = f.poll()? {
                    let acc = fold(acc, v);
                    if let Some(p) = iter.next() {
                        self.0 = Phase::A((p.lossless_write_to(w), iter, acc, fold));
                        self.poll()
                    } else {
                        Ok(Async::Ready((w, acc)))
                    }
                } else {
                    self.0 = Phase::A((f, iter, acc, fold));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((w, acc)) => Ok(Async::Ready((w, acc))),
            _ => panic!("Cannot poll WriteIterFold twice"),
        }
    }
}
impl<W: Write, I, F, T> WriteTo<W> for pattern::combinators::IterFold<I, F, T>
    where I: Iterator,
          I::Item: WriteTo<W>,
          F: Fn(T, <I::Item as Pattern>::Value) -> T
{
    type Future = WriteIterFold<W, I::Item, I, F, T>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        let (mut iter, fold, init) = self.unwrap();
        if let Some(p) = iter.next() {
            WriteIterFold(Phase::A((p.lossless_write_to(writer), iter, init, fold)))
        } else {
            WriteIterFold(Phase::B((writer, init)))
        }
    }
}

/// A future for writing `Iter` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::WriteTo;
/// use handy_io::pattern::{Pattern, Iter};
///
/// let iter_fold_pattern = Iter(vec![0u8, 1, 2].into_iter());
/// let mut buf = [0; 3];
/// let value = iter_fold_pattern.sync_write_to(&mut &mut buf[..]).unwrap();
/// assert_eq!(buf, [0, 1, 2]);
/// ```
pub type WriteIter<W, I>
    where I: Iterator,
          I::Item: Pattern = WriteIterFold<W,
                                           I::Item,
                                           I,
                                           fn((), <I::Item as Pattern>::Value) -> (),
                                           ()>;
impl<W: Write, I> WriteTo<W> for pattern::Iter<I>
    where I: Iterator,
          I::Item: WriteTo<W>
{
    type Future = WriteIter<W, I>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        fn fold<T>(_: (), _: T) -> () {
            ()
        }
        self.fold((), fold as _).lossless_write_to(writer)
    }
}

/// A future for writing `Option` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::WriteTo;
/// use handy_io::pattern::Pattern;
///
/// let mut buf = [0];
///
/// let none_pattern: Option<u8> = None;
/// none_pattern.sync_write_to(&mut &mut buf[..]).unwrap();
/// assert_eq!(buf, [0]);
///
/// let some_pattern: Option<u8> = Some(3);
/// some_pattern.sync_write_to(&mut &mut buf[..]).unwrap();
/// assert_eq!(buf, [3]);
/// ```
pub type WriteOption<W, P> where P: Pattern =
    <Branch<pattern::combinators::Map<P, fn(P::Value) -> Option<P::Value>>,
           io::Result<Option<P::Value>>> as WriteTo<W>>::Future;
impl<W: Write, P> WriteTo<W> for Option<P>
    where P: WriteTo<W>
{
    type Future = WriteOption<W, P>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        if let Some(p) = self {
                Branch::A(p.map(Some as fn(P::Value) -> Option<P::Value>))
            } else {
                Branch::B(Ok(None)) as Branch<_, _>
            }
            .lossless_write_to(writer)
    }
}

/// A future for writing `Result` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::WriteTo;
/// use handy_io::pattern::Pattern;
///
/// let mut buf = [];
/// let ok_pattern = Ok("value");
/// let value = ok_pattern.sync_write_to(&mut &mut buf[..]).unwrap();
/// assert_eq!(value, "value");
/// ```
pub type WriteResult<W, T> = futures::Done<(W, T), (W, io::Error)>;
impl<W: Write, T> WriteTo<W> for io::Result<T> {
    type Future = WriteResult<W, T>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        futures::done(match self {
            Ok(v) => Ok((writer, v)),
            Err(e) => Err((writer, e)),
        })
    }
}
