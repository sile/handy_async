use std::io::{self, Read};
use futures::{self, Future, Poll, Async};

use pattern::{self, Pattern, Branch};
use super::ReadFrom;
use super::super::common::{self, Phase};

/// A future for reading `Then` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::Pattern;
/// use handy_io::pattern::read::U8;
///
/// let then_pattern = U8.then(|r| match r { Ok(0) => Ok(true), _ => Ok(false) } );
///
/// assert!(!then_pattern.sync_read_from(&mut &[1][..]).unwrap());
/// ```
pub struct ReadThen<R: Read, P0, P1, F>(Phase<(P0::Future, F), P1::Future>)
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>;
impl<R: Read, P0, P1, F> Future for ReadThen<R, P0, P1, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>,
          F: FnOnce(io::Result<P0::Value>) -> P1
{
    type Item = (R, P1::Value);
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut f, then)) => {
                match f.poll() {
                    Err((r, e)) => {
                        self.0 = Phase::B(then(Err(e)).lossless_read_from(r));
                        self.poll()
                    }
                    Ok(Async::Ready((r, v0))) => {
                        self.0 = Phase::B(then(Ok(v0)).lossless_read_from(r));
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
            _ => panic!("Cannot poll ReadThen twice"),
        }
    }
}
impl<R: Read, P0, P1, F> ReadFrom<R> for pattern::combinators::Then<P0, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>,
          F: FnOnce(io::Result<P0::Value>) -> P1
{
    type Future = ReadThen<R, P0, P1, F>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        let (p, f) = self.unwrap();
        ReadThen(Phase::A((p.lossless_read_from(reader), f)))
    }
}

/// A future for reading `AndThen` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::Pattern;
/// use handy_io::pattern::read::U8;
///
/// let and_then_pattern = U8.and_then(|b| if b == 0 { Ok(true) } else { Ok(false) } );
///
/// assert!(!and_then_pattern.sync_read_from(&mut &[1][..]).unwrap());
/// ```
pub type ReadAndThen<R, P0, P1, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>,
          F: FnOnce(P0::Value) -> P1,
          R: Read = ReadThen<R,
                             (P0, io::Result<F>),
                             Branch<P1, io::Result<P1::Value>>,
                             fn(io::Result<(P0::Value, F)>) -> Branch<P1, io::Result<P1::Value>>>;
impl<R: Read, P0, P1, F> ReadFrom<R> for pattern::combinators::AndThen<P0, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>,
          F: FnOnce(P0::Value) -> P1
{
    type Future = ReadAndThen<R, P0, P1, F>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        let (p0, and_then) = self.unwrap();
        (p0, Ok(and_then))
            .then(common::then_to_and_then as _)
            .lossless_read_from(reader)
    }
}

/// A future for reading `OrElse` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::Pattern;
/// use handy_io::pattern::read::U8;
///
/// let or_else_pattern = U8.or_else(|e| Ok(0xFF));
///
/// assert_eq!(or_else_pattern.sync_read_from(std::io::empty()).unwrap(), 0xFF);
/// ```
pub struct ReadOrElse<R: Read, P0, P1, F>(Phase<(P0::Future, F), P1::Future>)
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>;
impl<R: Read, P0, P1, F> Future for ReadOrElse<R, P0, P1, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R, Value = P0::Value>,
          F: FnOnce(io::Error) -> P1
{
    type Item = (R, P1::Value);
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut f, or_else)) => {
                match f.poll() {
                    Err((r, e)) => {
                        self.0 = Phase::B(or_else(e).lossless_read_from(r));
                        self.poll()
                    }
                    Ok(Async::Ready((r, v0))) => Ok(Async::Ready((r, v0))),
                    Ok(Async::NotReady) => {
                        self.0 = Phase::A((f, or_else));
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
            _ => panic!("Cannot poll ReadOrElse twice"),
        }
    }
}
impl<R: Read, P0, P1, F> ReadFrom<R> for pattern::combinators::OrElse<P0, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R, Value = P0::Value>,
          F: FnOnce(io::Error) -> P1
{
    type Future = ReadOrElse<R, P0, P1, F>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        let (p, f) = self.unwrap();
        ReadOrElse(Phase::A((p.lossless_read_from(reader), f)))
    }
}

/// A future for reading `Map` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::Pattern;
/// use handy_io::pattern::read::U8;
///
/// let map_pattern = U8.map(|b| b * 2);
///
/// assert_eq!(map_pattern.sync_read_from(&mut &[2][..]).unwrap(), 4);
/// ```
pub struct ReadMap<R, P, F, T>(ReadMapInner<R, P, F, T>)
    where P: ReadFrom<R>,
          F: FnOnce(P::Value) -> T,
          R: Read;
impl<R, P, F, T> Future for ReadMap<R, P, F, T>
    where P: ReadFrom<R>,
          F: FnOnce(P::Value) -> T,
          R: Read
{
    type Item = (R, T);
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}
type ReadMapInner<R, P, F, T>
    where P: ReadFrom<R>,
          F: FnOnce(P::Value) -> T,
          R: Read = ReadAndThen<R,
                                (P, io::Result<F>),
                                io::Result<T>,
                                fn((P::Value, F)) -> io::Result<T>>;
impl<R: Read, P, F, T> ReadFrom<R> for pattern::combinators::Map<P, F>
    where P: ReadFrom<R>,
          F: FnOnce(P::Value) -> T
{
    type Future = ReadMap<R, P, F, T>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        let (p, f) = self.unwrap();
        ReadMap((p, Ok(f)).and_then(common::and_then_to_map as _).lossless_read_from(reader))
    }
}

/// A future for reading `Chain` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::Pattern;
/// use handy_io::pattern::read::U8;
///
/// let chain_pattern = U8.chain(U8);
///
/// assert_eq!(chain_pattern.sync_read_from(&mut &[1, 2][..]).unwrap(), (1, 2));
/// ```
pub struct ReadChain<R: Read, P0, P1>(Phase<(P0::Future, P1), (P1::Future, P0::Value)>)
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>;
impl<R: Read, P0, P1> Future for ReadChain<R, P0, P1>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>
{
    type Item = (R, (P0::Value, P1::Value));
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut f, p0)) => {
                if let Async::Ready((r, v0)) = f.poll()? {
                    self.0 = Phase::B((p0.lossless_read_from(r), v0));
                    self.poll()
                } else {
                    self.0 = Phase::A((f, p0));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((mut f, v0)) => {
                if let Async::Ready((r, v1)) = f.poll()? {
                    Ok(Async::Ready((r, (v0, v1))))
                } else {
                    self.0 = Phase::B((f, v0));
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll ReadChain twice"),
        }
    }
}
impl<R: Read, P0, P1> ReadFrom<R> for pattern::combinators::Chain<P0, P1>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>
{
    type Future = ReadChain<R, P0, P1>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        let (p0, p1) = self.unwrap();
        ReadChain(Phase::A((p0.lossless_read_from(reader), p1)))
    }
}

/// A future for reading `IterFold` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::{Pattern, Iter};
/// use handy_io::pattern::read::U8;
///
/// let iter_fold_pattern = Iter(vec![U8, U8].into_iter()).fold(0, |acc, b| acc + b);
///
/// assert_eq!(iter_fold_pattern.sync_read_from(&mut &[1, 2][..]).unwrap(), 3);
/// ```
pub struct ReadIterFold<R: Read, P, I, F, T>(Phase<(P::Future, I, T, F), (R, T)>)
    where P: ReadFrom<R>;
impl<R: Read, I, F, T> Future for ReadIterFold<R, I::Item, I, F, T>
    where I: Iterator,
          I::Item: ReadFrom<R>,
          F: Fn(T, <I::Item as Pattern>::Value) -> T
{
    type Item = (R, T);
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut future, mut iter, acc, fold)) => {
                if let Async::Ready((r, v)) = future.poll()? {
                    let acc = fold(acc, v);
                    if let Some(p) = iter.next() {
                        self.0 = Phase::A((p.lossless_read_from(r), iter, acc, fold));
                        self.poll()
                    } else {
                        Ok(Async::Ready((r, acc)))
                    }
                } else {
                    self.0 = Phase::A((future, iter, acc, fold));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((r, t)) => Ok(Async::Ready((r, t))),
            _ => panic!("Cannot poll ReadIterFold twice"),
        }
    }
}
impl<R: Read, I, F, T> ReadFrom<R> for pattern::combinators::IterFold<I, F, T>
    where I: Iterator,
          I::Item: ReadFrom<R>,
          F: Fn(T, <I::Item as Pattern>::Value) -> T
{
    type Future = ReadIterFold<R, I::Item, I, F, T>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        let (mut iter, fold, init) = self.unwrap();
        if let Some(p) = iter.next() {
            ReadIterFold(Phase::A((p.lossless_read_from(reader), iter, init, fold)))
        } else {
            ReadIterFold(Phase::B((reader, init)))
        }
    }
}

/// A future for reading `Iter` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::{Pattern, Iter};
/// use handy_io::pattern::read::U8;
///
/// let iter_pattern = Iter(vec![U8, U8].into_iter());
///
/// assert_eq!(iter_pattern.sync_read_from(&mut &[1, 2][..]).unwrap(), ());
/// ```
pub type ReadIter<R, I>
    where I: Iterator,
          I::Item: ReadFrom<R>,
          R: Read = ReadIterFold<R, I::Item, I, fn((), <I::Item as Pattern>::Value) -> (), ()>;
impl<R: Read, I> ReadFrom<R> for pattern::Iter<I>
    where I: Iterator,
          I::Item: ReadFrom<R>
{
    type Future = ReadIter<R, I>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        fn f<T>(():(), _: T) -> () {
            ()
        }
        self.fold((), f as _).lossless_read_from(reader)
    }
}

/// A future for reading `Option` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::Pattern;
/// use handy_io::pattern::read::U8;
///
/// let none_pattern: Option<U8> = None;
/// assert_eq!(none_pattern.sync_read_from(&mut &[1, 2][..]).unwrap(), None);
///
/// let some_pattern: Option<U8> = Some(U8);
/// assert_eq!(some_pattern.sync_read_from(&mut &[1, 2][..]).unwrap(), Some(1));
/// ```
pub type ReadOption<R, P>
    where R: Read,
          P: ReadFrom<R> = Branch<ReadMap<R,
                                          P,
                                          fn(P::Value) -> Option<P::Value>,
                                          Option<P::Value>>,
                                  futures::Done<(R, Option<P::Value>), (R, io::Error)>>;
impl<R: Read, P> ReadFrom<R> for Option<P>
    where P: ReadFrom<R>
{
    type Future = ReadOption<R, P>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        let pattern = if let Some(p) = self {
            Branch::A(p.map(Some as _))
        } else {
            Branch::B(Ok(None)) as Branch<_, _>
        };
        pattern.lossless_read_from(reader)
    }
}

/// A future for reading `Result` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::Pattern;
///
/// let ok_pattern = Ok(5);
/// assert_eq!(ok_pattern.sync_read_from(std::io::empty()).unwrap(), 5);
/// ```
pub type ReadResult<R, T> = futures::Done<(R, T), (R, io::Error)>;
impl<R: Read, T> ReadFrom<R> for io::Result<T> {
    type Future = ReadResult<R, T>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        futures::done(match self {
            Ok(v) => Ok((reader, v)),
            Err(e) => Err((reader, e)),
        })
    }
}

impl<R: Read> ReadFrom<R> for () {
    type Future = futures::Finished<(R, ()), (R, io::Error)>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        futures::finished((reader, ()))
    }
}

impl<R: Read, P0, P1> ReadFrom<R> for (P0, P1)
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>
{
    type Future = ReadChain<R, P0, P1>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        self.0.chain(self.1).lossless_read_from(reader)
    }
}

type MapFuture<F, T> where F: Future = futures::Map<F, fn(F::Item) -> T>;
macro_rules! impl_tuple_read_from {
    ([$($p:ident),* | $pn:ident], [$($i:tt),* | $it:tt]) => {
        impl<R: Read, $($p),*, $pn> ReadFrom<R> for ($($p),*, $pn)
            where $($p:ReadFrom<R>,)*
                  $pn:ReadFrom<R>
        {
            type Future = MapFuture<<(($($p),*), $pn) as ReadFrom<R>>::Future,
            (R, ($($p::Value),*,$pn::Value))>;
            fn lossless_read_from(self, reader: R) -> Self::Future {
                fn flatten<R, $($p),*, $pn>((r, (a, b)): (R, (($($p),*), $pn))) ->
                    (R, ($($p),*, $pn)) {
                    (r, ($(a.$i),*, b))
                }
                (($(self.$i),*), self.$it).lossless_read_from(reader).map(flatten as _)
            }
        }
    }
}
impl_tuple_read_from!([P0, P1 | P2], [0, 1 | 2]);
impl_tuple_read_from!([P0, P1, P2 | P3], [0, 1, 2 | 3]);
impl_tuple_read_from!([P0, P1, P2, P3 | P4], [0, 1, 2, 3 | 4]);
impl_tuple_read_from!([P0, P1, P2, P3, P4 | P5], [0, 1, 2, 3, 4 | 5]);
impl_tuple_read_from!([P0, P1, P2, P3, P4, P5 | P6], [0, 1, 2, 3, 4, 5 | 6]);
impl_tuple_read_from!([P0, P1, P2, P3, P4, P5, P6 | P7], [0, 1, 2, 3, 4, 5, 6 | 7]);
impl_tuple_read_from!([P0, P1, P2, P3, P4, P5, P6, P7 | P8],
                      [0, 1, 2, 3, 4, 5, 6, 7 | 8]);
impl_tuple_read_from!([P0, P1, P2, P3, P4, P5, P6, P7, P8 | P9],
                      [0, 1, 2, 3, 4, 5, 6, 7, 8 | 9]);

/// A future for reading `Branch` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::{Pattern, Branch};
/// use handy_io::pattern::read::{U8, I8};
///
/// let branch_pattern = U8.and_then(|b| {
///     if  b % 2 == 0 { Branch::A(U8) as Branch<_, _> } else { Branch::B(Ok(0u8)) }
/// });
/// assert_eq!(branch_pattern.sync_read_from(&mut &[0, 0xFF][..]).unwrap(), 0xFFu8);
/// ```
pub type ReadBranch<R, A, B, C, D, E, F, G, H>
    where A: ReadFrom<R>,
          B: ReadFrom<R, Value = A::Value>,
          C: ReadFrom<R, Value = A::Value>,
          D: ReadFrom<R, Value = A::Value>,
          E: ReadFrom<R, Value = A::Value>,
          F: ReadFrom<R, Value = A::Value>,
          G: ReadFrom<R, Value = A::Value>,
          H: ReadFrom<R, Value = A::Value>,
          R: Read = Branch<A::Future,
                           B::Future,
                           C::Future,
                           D::Future,
                           E::Future,
                           F::Future,
                           G::Future,
                           H::Future>;
impl<R: Read, A, B, C, D, E, F, G, H> ReadFrom<R> for Branch<A, B, C, D, E, F, G, H>
    where A: ReadFrom<R>,
          B: ReadFrom<R, Value = A::Value>,
          C: ReadFrom<R, Value = A::Value>,
          D: ReadFrom<R, Value = A::Value>,
          E: ReadFrom<R, Value = A::Value>,
          F: ReadFrom<R, Value = A::Value>,
          G: ReadFrom<R, Value = A::Value>,
          H: ReadFrom<R, Value = A::Value>
{
    type Future = ReadBranch<R, A, B, C, D, E, F, G, H>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        match self {
            Branch::A(p) => Branch::A(p.lossless_read_from(reader)),
            Branch::B(p) => Branch::B(p.lossless_read_from(reader)),
            Branch::C(p) => Branch::C(p.lossless_read_from(reader)),
            Branch::D(p) => Branch::D(p.lossless_read_from(reader)),
            Branch::E(p) => Branch::E(p.lossless_read_from(reader)),
            Branch::F(p) => Branch::F(p.lossless_read_from(reader)),
            Branch::G(p) => Branch::G(p.lossless_read_from(reader)),
            Branch::H(p) => Branch::H(p.lossless_read_from(reader)),
        }
    }
}

#[cfg(test)]
mod test {
    use std::io;
    use futures::Future;

    use pattern::{self, Pattern};
    use super::super::*;

    #[test]
    fn it_works() {
        assert_eq!(().and_then(|_| ())
                       .map(|_| 10)
                       .lossless_read_from(io::Cursor::new(vec![]))
                       .wait()
                       .unwrap()
                       .1,
                   10);

        let pattern = pattern::Iter(vec![(), (), ()].into_iter()).fold(0, |n, ()| n + 1);
        assert_eq!(pattern.lossless_read_from(io::Cursor::new(vec![])).wait().unwrap().1,
                   3);
    }
}
