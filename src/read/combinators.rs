use std;
use std::mem;
use std::io::{self, Read};
use futures::{self, Future, Poll, Async};

use pattern::{self, Pattern};
use super::ReadFrom;

pub enum ReadMap<R: Read, P, F>
    where P: ReadFrom<R>
{
    State(P::Future, F),
    Polled,
}
impl<R: Read, P, F, T> Future for ReadMap<R, P, F>
    where P: ReadFrom<R>,
          F: FnOnce(P::Value) -> T
{
    type Item = (R, T);
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let ReadMap::State(mut future, map) = mem::replace(self, ReadMap::Polled) {
            if let Async::Ready((r, v)) = future.poll()? {
                Ok(Async::Ready((r, map(v))))
            } else {
                *self = ReadMap::State(future, map);
                Ok(Async::NotReady)
            }
        } else {
            panic!("Cannot poll ReadMap twice");
        }
    }
}
impl<R: Read, P, F, T> ReadFrom<R> for pattern::Map<P, F>
    where P: ReadFrom<R>,
          F: FnOnce(P::Value) -> T
{
    type Future = ReadMap<R, P, F>;
    fn read_from(self, reader: R) -> Self::Future {
        let (p, f) = self.unwrap();
        ReadMap::State(p.read_from(reader), f)
    }
}

pub enum ReadThen<R: Read, P0, P1, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>
{
    Phase0(P0::Future, F),
    Phase1(P1::Future),
    Polled,
}
impl<R: Read, P0, P1, F> Future for ReadThen<R, P0, P1, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>,
          F: FnOnce(io::Result<P0::Value>) -> P1
{
    type Item = (R, P1::Value);
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match mem::replace(self, ReadThen::Polled) {
            ReadThen::Phase0(mut f, then) => {
                match f.poll() {
                    Err((r, e)) => {
                        *self = ReadThen::Phase1(then(Err(e)).read_from(r));
                        self.poll()
                    }
                    Ok(Async::Ready((r, v0))) => {
                        *self = ReadThen::Phase1(then(Ok(v0)).read_from(r));
                        self.poll()
                    }
                    Ok(Async::NotReady) => {
                        *self = ReadThen::Phase0(f, then);
                        Ok(Async::NotReady)
                    }
                }
            }
            ReadThen::Phase1(mut f) => {
                let result = f.poll()?;
                if let Async::NotReady = result {
                    *self = ReadThen::Phase1(f);
                }
                Ok(result)
            }
            ReadThen::Polled => panic!("Cannot poll ReadThen twice"),
        }
    }
}
impl<R: Read, P0, P1, F> ReadFrom<R> for pattern::Then<P0, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>,
          F: FnOnce(io::Result<P0::Value>) -> P1
{
    type Future = ReadThen<R, P0, P1, F>;
    fn read_from(self, reader: R) -> Self::Future {
        let (p, f) = self.unwrap();
        ReadThen::Phase0(p.read_from(reader), f)
    }
}

pub enum ReadOrElse<R: Read, P0, P1, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>
{
    Phase0(P0::Future, F),
    Phase1(P1::Future),
    Polled,
}
impl<R: Read, P0, P1, F> Future for ReadOrElse<R, P0, P1, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R, Value = P0::Value>,
          F: FnOnce(io::Error) -> P1
{
    type Item = (R, P1::Value);
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match mem::replace(self, ReadOrElse::Polled) {
            ReadOrElse::Phase0(mut f, or_else) => {
                match f.poll() {
                    Err((r, e)) => {
                        *self = ReadOrElse::Phase1(or_else(e).read_from(r));
                        self.poll()
                    }
                    Ok(Async::Ready((r, v0))) => Ok(Async::Ready((r, v0))),
                    Ok(Async::NotReady) => {
                        *self = ReadOrElse::Phase0(f, or_else);
                        Ok(Async::NotReady)
                    }
                }
            }
            ReadOrElse::Phase1(mut f) => {
                let result = f.poll()?;
                if let Async::NotReady = result {
                    *self = ReadOrElse::Phase1(f);
                }
                Ok(result)
            }
            ReadOrElse::Polled => panic!("Cannot poll ReadOrElse twice"),
        }
    }
}
impl<R: Read, P0, P1, F> ReadFrom<R> for pattern::OrElse<P0, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R, Value = P0::Value>,
          F: FnOnce(io::Error) -> P1
{
    type Future = ReadOrElse<R, P0, P1, F>;
    fn read_from(self, reader: R) -> Self::Future {
        let (p, f) = self.unwrap();
        ReadOrElse::Phase0(p.read_from(reader), f)
    }
}

pub enum ReadAndThen<R: Read, P0, P1, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>
{
    Phase0(P0::Future, F),
    Phase1(P1::Future),
    Polled,
}
impl<R: Read, P0, P1, F> Future for ReadAndThen<R, P0, P1, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>,
          F: FnOnce(P0::Value) -> P1
{
    type Item = (R, P1::Value);
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match mem::replace(self, ReadAndThen::Polled) {
            ReadAndThen::Phase0(mut f, and_then) => {
                if let Async::Ready((r, v0)) = f.poll()? {
                    *self = ReadAndThen::Phase1(and_then(v0).read_from(r));
                    self.poll()
                } else {
                    *self = ReadAndThen::Phase0(f, and_then);
                    Ok(Async::NotReady)
                }
            }
            ReadAndThen::Phase1(mut f) => {
                let result = f.poll()?;
                if let Async::NotReady = result {
                    *self = ReadAndThen::Phase1(f);
                }
                Ok(result)
            }
            ReadAndThen::Polled => panic!("Cannot poll ReadAndHhen twice"),
        }
    }
}
impl<R: Read, P0, P1, F> ReadFrom<R> for pattern::AndThen<P0, F>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>,
          F: FnOnce(P0::Value) -> P1
{
    type Future = ReadAndThen<R, P0, P1, F>;
    fn read_from(self, reader: R) -> Self::Future {
        let (p, f) = self.unwrap();
        ReadAndThen::Phase0(p.read_from(reader), f)
    }
}

pub enum ReadChain<R: Read, P0, P1>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>
{
    Phase0(P0::Future, P1),
    Phase1(P1::Future, P0::Value),
    Polled,
}
impl<R: Read, P0, P1> Future for ReadChain<R, P0, P1>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>
{
    type Item = (R, (P0::Value, P1::Value));
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match mem::replace(self, ReadChain::Polled) {
            ReadChain::Phase0(mut f, p0) => {
                if let Async::Ready((r, v0)) = f.poll()? {
                    *self = ReadChain::Phase1(p0.read_from(r), v0);
                    self.poll()
                } else {
                    *self = ReadChain::Phase0(f, p0);
                    Ok(Async::NotReady)
                }
            }
            ReadChain::Phase1(mut f, v0) => {
                if let Async::Ready((r, v1)) = f.poll()? {
                    Ok(Async::Ready((r, (v0, v1))))
                } else {
                    *self = ReadChain::Phase1(f, v0);
                    Ok(Async::NotReady)
                }
            }
            ReadChain::Polled => panic!("Cannot poll ReadChain twice"),
        }
    }
}
impl<R: Read, P0, P1> ReadFrom<R> for pattern::Chain<P0, P1>
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>
{
    type Future = ReadChain<R, P0, P1>;
    fn read_from(self, reader: R) -> Self::Future {
        let (p0, p1) = self.unwrap();
        ReadChain::Phase0(p0.read_from(reader), p1)
    }
}

pub enum ReadIterFold<R: Read, P, I, F, T>
    where P: ReadFrom<R>
{
    State(P::Future, I, T, F),
    Empty(R, T),
    Polled,
}
impl<R: Read, I, F, T> Future for ReadIterFold<R, I::Item, I, F, T>
    where I: Iterator,
          I::Item: ReadFrom<R>,
          F: Fn(T, <I::Item as Pattern>::Value) -> T
{
    type Item = (R, T);
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match mem::replace(self, ReadIterFold::Polled) {
            ReadIterFold::State(mut future, mut iter, acc, fold) => {
                if let Async::Ready((r, v)) = future.poll()? {
                    let acc = fold(acc, v);
                    if let Some(p) = iter.next() {
                        *self = ReadIterFold::State(p.read_from(r), iter, acc, fold);
                        self.poll()
                    } else {
                        Ok(Async::Ready((r, acc)))
                    }
                } else {
                    *self = ReadIterFold::State(future, iter, acc, fold);
                    Ok(Async::NotReady)
                }
            }
            ReadIterFold::Empty(r, t) => Ok(Async::Ready((r, t))),
            ReadIterFold::Polled => panic!("Cannot poll ReadIterFold twice"),
        }
    }
}
impl<R: Read, I, F, T> ReadFrom<R> for pattern::IterFold<I, F, T>
    where I: Iterator,
          I::Item: ReadFrom<R>,
          F: Fn(T, <I::Item as Pattern>::Value) -> T
{
    type Future = ReadIterFold<R, I::Item, I, F, T>;
    fn read_from(self, reader: R) -> Self::Future {
        let (mut iter, fold, init) = self.unwrap();
        if let Some(p) = iter.next() {
            ReadIterFold::State(p.read_from(reader), iter, init, fold)
        } else {
            ReadIterFold::Empty(reader, init)
        }
    }
}
impl<R: Read, I> ReadFrom<R> for pattern::Iter<I>
    where I: Iterator,
          I::Item: ReadFrom<R>
{
    type Future = ReadIterFold<R, I::Item, I, fn((), <I::Item as Pattern>::Value) -> (), ()>;
    fn read_from(self, reader: R) -> Self::Future {
        fn f<T>(():(), _: T) -> () {
            ()
        }
        self.fold((), f as _).read_from(reader)
    }
}

pub type ReadOption<R, P>
    where R: Read,
          P: ReadFrom<R> = ReadIterFold<R,
                                        P,
                                        std::option::IntoIter<P>,
                                        fn(Option<P::Value>, P::Value) -> Option<P::Value>,
                                        Option<P::Value>>;

impl<R: Read, P> ReadFrom<R> for Option<P>
    where P: ReadFrom<R>
{
    type Future = ReadOption<R, P>;
    fn read_from(self, reader: R) -> Self::Future {
        fn fold<T>(_: Option<T>, v: T) -> Option<T> {
            Some(v)
        }
        pattern::iter(self.into_iter()).fold(None, fold as _).read_from(reader)
    }
}

impl<R: Read, T> ReadFrom<R> for io::Result<T> {
    type Future = futures::Done<(R, T), (R, io::Error)>;
    fn read_from(self, reader: R) -> Self::Future {
        futures::done(match self {
            Ok(v) => Ok((reader, v)),
            Err(e) => Err((reader, e)),
        })
    }
}

impl<R: Read> ReadFrom<R> for () {
    type Future = futures::Finished<(R, ()), (R, io::Error)>;
    fn read_from(self, reader: R) -> Self::Future {
        futures::finished((reader, ()))
    }
}

impl<R: Read, P0, P1> ReadFrom<R> for (P0, P1)
    where P0: ReadFrom<R>,
          P1: ReadFrom<R>
{
    type Future = ReadChain<R, P0, P1>;
    fn read_from(self, reader: R) -> Self::Future {
        self.0.chain(self.1).read_from(reader)
    }
}

pub type MapFuture<F, T> where F: Future = futures::Map<F, fn(F::Item) -> T>;

macro_rules! impl_tuple_read_from {
    ([$($p:ident),* | $pn:ident], [$($i:tt),* | $it:tt]) => {
        impl<R: Read, $($p),*, $pn> ReadFrom<R> for ($($p),*, $pn)
            where $($p:ReadFrom<R>,)*
                  $pn:ReadFrom<R>
        {
            type Future = MapFuture<<(($($p),*), $pn) as ReadFrom<R>>::Future,
            (R, ($($p::Value),*,$pn::Value))>;
            fn read_from(self, reader: R) -> Self::Future {
                fn flatten<R, $($p),*, $pn>((r, (a, b)): (R, (($($p),*), $pn))) ->
                    (R, ($($p),*, $pn)) {
                    (r, ($(a.$i),*, b))
                }
                (($(self.$i),*), self.$it).read_from(reader).map(flatten as _)
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

pub enum ReadBranch<P0, P1, P2 = P0, P3 = P0, P4 = P0, P5 = P0, P6 = P0, P7 = P0> {
    P0(P0),
    P1(P1),
    P2(P2),
    P3(P3),
    P4(P4),
    P5(P5),
    P6(P6),
    P7(P7),
    Polled,
}
impl<P0, P1, P2, P3, P4, P5, P6, P7> Future for ReadBranch<P0, P1, P2, P3, P4, P5, P6, P7>
    where P0: Future,
          P1: Future<Item = P0::Item, Error = P0::Error>,
          P2: Future<Item = P0::Item, Error = P0::Error>,
          P3: Future<Item = P0::Item, Error = P0::Error>,
          P4: Future<Item = P0::Item, Error = P0::Error>,
          P5: Future<Item = P0::Item, Error = P0::Error>,
          P6: Future<Item = P0::Item, Error = P0::Error>,
          P7: Future<Item = P0::Item, Error = P0::Error>
{
    type Item = P0::Item;
    type Error = P0::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        fn poll_and_on_not_ready<F, T>(mut f: F, t: T) -> Poll<F::Item, F::Error>
            where F: Future,
                  T: FnOnce(F)
        {
            f.poll().map(|r| {
                if let Async::NotReady = r {
                    t(f);
                };
                r
            })
        }
        match mem::replace(self, ReadBranch::Polled) {
            ReadBranch::P0(f) => poll_and_on_not_ready(f, |f| *self = ReadBranch::P0(f)),
            ReadBranch::P1(f) => poll_and_on_not_ready(f, |f| *self = ReadBranch::P1(f)),
            ReadBranch::P2(f) => poll_and_on_not_ready(f, |f| *self = ReadBranch::P2(f)),
            ReadBranch::P3(f) => poll_and_on_not_ready(f, |f| *self = ReadBranch::P3(f)),
            ReadBranch::P4(f) => poll_and_on_not_ready(f, |f| *self = ReadBranch::P4(f)),
            ReadBranch::P5(f) => poll_and_on_not_ready(f, |f| *self = ReadBranch::P5(f)),
            ReadBranch::P6(f) => poll_and_on_not_ready(f, |f| *self = ReadBranch::P6(f)),
            ReadBranch::P7(f) => poll_and_on_not_ready(f, |f| *self = ReadBranch::P7(f)),
            ReadBranch::Polled => panic!("Cannot poll ReadBranch twice"),
        }
    }
}

macro_rules! impl_branch_read_from {
    ($b:ident, P0, $($p:ident),*) => {
        impl<R: Read, P0, $($p),*> ReadFrom<R> for pattern::$b<P0, $($p),*>
            where P0: ReadFrom<R>, $($p: ReadFrom<R, Value = P0::Value>),*
        {
            type Future = ReadBranch<P0::Future, $($p::Future),*>;
            fn read_from(self, reader: R) -> Self::Future {
                match self {
                    pattern::$b::P0(p) => ReadBranch::P0(p.read_from(reader)),
                    $(pattern::$b::$p(p) => ReadBranch::$p(p.read_from(reader))),*
                }
            }
        }
    }
}

impl_branch_read_from!(Branch2, P0, P1);
impl_branch_read_from!(Branch3, P0, P1, P2);
impl_branch_read_from!(Branch4, P0, P1, P2, P3);
impl_branch_read_from!(Branch5, P0, P1, P2, P3, P4);
impl_branch_read_from!(Branch6, P0, P1, P2, P3, P4, P5);
impl_branch_read_from!(Branch7, P0, P1, P2, P3, P4, P5, P6);
impl_branch_read_from!(Branch8, P0, P1, P2, P3, P4, P5, P6, P7);

#[cfg(test)]
mod test {
    use std::io;
    use futures::Future;

    use pattern::{self, Pattern};
    use super::super::*;

    #[test]
    fn it_works() {
        ().and_then(|_| ()).read_from(io::Cursor::new(vec![])).wait().unwrap();

        let pattern = pattern::iter(vec![(), (), ()].into_iter()).fold(0, |n, ()| n + 1);
        assert_eq!(pattern.read_from(io::Cursor::new(vec![])).wait().unwrap().1,
                   3);
    }
}
