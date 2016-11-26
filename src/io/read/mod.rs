use std::io::{self, Read};
use std::fmt;
use futures::{self, Poll, Async, Future};

use pattern::{Pattern, Window};
use super::futures::IoFuture;
use super::common::Phase;

pub mod primitives;
pub mod combinators;

/// The `ReadFrom` trait allows for reading
/// a value of the pattern from a source asynchronously.
///
/// # Notice
///
/// For executing asynchronously, we assume the reader `R`
/// returns the `std::io::ErrorKind::WouldBlock` error
/// if a read operation would be about to block.
pub trait ReadFrom<R: Read>: Pattern {
    /// The future to read a value of the pattern from `R`.
    type Future: Future<Item = (R, Self::Value), Error = (R, io::Error)>;

    /// Creates a future instance to read a value of the pattern from `reader`.
    fn lossless_read_from(self, reader: R) -> Self::Future;

    /// Creates a future instance to read a value of the pattern from `reader`.
    ///
    /// If the execution of the future fails, the `reader` will be dropped.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate futures;
    /// extern crate handy_io;
    ///
    /// use handy_io::io::ReadFrom;
    /// use handy_io::pattern::{Pattern, Endian};
    /// use handy_io::pattern::read::{U8, U16};
    /// use futures::Future;
    ///
    /// fn main() {
    ///     let mut input = &[1, 0, 2][..];
    ///     let pattern = (U8, U16.be());
    ///     let future = pattern.read_from(&mut input);
    ///     assert_eq!(future.wait().unwrap().1, (1, 2));
    /// }
    /// ```
    fn read_from(self, reader: R) -> LossyReadFrom<R, Self::Future> {
        fn conv<R>((_, e): (R, io::Error)) -> io::Error {
            e
        }
        self.lossless_read_from(reader).map_err(conv as _)
    }

    /// Scynchronously reading a value of the pattern from `reader`.
    ///
    /// # Example
    ///
    /// ```
    /// use handy_io::io::ReadFrom;
    /// use handy_io::pattern::{Pattern, Endian};
    /// use handy_io::pattern::read::{U8, U16};
    ///
    /// let pattern = (U8, U16.be());
    /// assert_eq!(pattern.sync_read_from(&mut &[1, 0, 2][..]).unwrap(), (1, 2));
    /// ```
    fn sync_read_from(self, reader: R) -> io::Result<Self::Value> {
        self.lossless_read_from(reader).map(|(_, v)| v).map_err(|(_, e)| e).wait()
    }

    /// Returns the boxed version of this pattern.
    fn boxed(self) -> BoxReadFrom<R, Self::Value>
        where Self: Send + 'static,
              Self::Future: Send + 'static
    {
        let mut f = Some(move |reader: R| self.lossless_read_from(reader).boxed());
        BoxReadFrom(Box::new(move |reader| (f.take().unwrap())(reader)))
    }
}

/// The lossy version of the `ReadFrom` future `F`.
///
/// This future will drop `R`, if the execution fails.
pub type LossyReadFrom<R, F> = futures::MapErr<F, fn((R, io::Error)) -> io::Error>;

/// Boxed object which implements `ReadFrom` trait.
///
/// This object can be created with the `ReadFrom::boxed` method.
pub struct BoxReadFrom<R, T>(Box<FnMut(R) -> IoFuture<R, T>>);
impl<R, T> Pattern for BoxReadFrom<R, T> {
    type Value = T;
}
impl<R: Read, T> ReadFrom<R> for BoxReadFrom<R, T> {
    type Future = IoFuture<R, T>;
    fn lossless_read_from(mut self, reader: R) -> Self::Future {
        (self.0)(reader)
    }
}
impl<R, T> fmt::Debug for BoxReadFrom<R, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BoxReadfrom(..)")
    }
}

/// An asynchronous version of the standard `Read` trait.
///
/// # Notice
///
/// For executing asynchronously, we assume the reader which implements this trait
/// returns the `std::io::ErrorKind::WouldBlock` error
/// if a read operation would be about to block.
pub trait AsyncRead: io::Read + Sized {
    /// Creates a future which will read bytes asynchronously.
    fn async_read<B: AsMut<[u8]>>(self, buf: B) -> ReadBytes<Self, B> {
        ReadBytes(Some((self, buf)))
    }

    /// Creates a future which will read non empty bytes asynchronously.
    fn async_read_non_empty<B: AsMut<[u8]>>(self, buf: B) -> ReadNonEmpty<Self, B> {
        ReadNonEmpty(self.async_read(buf))
    }

    /// Creates a future which will read exact bytes asynchronously.
    fn async_read_exact<B: AsMut<[u8]>>(self, buf: B) -> ReadExact<Self, B> {
        ReadExact(self.async_read_non_empty(Window::new_mut(buf)))
    }

    /// Creates a future which will read bytes continuously and
    /// apply `f` on those to produce a final value.
    fn async_read_fold<F, B, T>(self, buf: B, init: T, f: F) -> ReadFold<Self, F, B, T>
        where F: Fn(T, B, usize) -> Result<(B, T), (B, io::Result<T>)>,
              B: AsMut<[u8]>
    {
        ReadFold(Some((self.async_read(buf), init, f)))
    }

    /// Creates a stream which will read values associated
    /// the patterns generated from `stream`.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate handy_io;
    /// extern crate futures;
    ///
    /// use handy_io::io::AsyncRead;
    /// use handy_io::pattern::read::U8;
    /// use futures::{Async, Stream};
    ///
    /// fn main() {
    ///     let patterns = futures::stream::iter(vec![Ok(U8), Ok(U8)].into_iter());
    ///     let mut reader = std::io::Cursor::new(vec![0, 1, 2]);
    ///     let mut stream = reader.async_read_stream(patterns);
    ///
    ///     assert_eq!(stream.poll().unwrap(), Async::Ready(Some(0)));
    ///     assert_eq!(stream.poll().unwrap(), Async::Ready(Some(1)));
    ///     assert_eq!(stream.poll().unwrap(), Async::Ready(None));
    /// }
    /// ```
    fn async_read_stream<S>(self, stream: S) -> ReadStream<Self, S>
        where S: futures::Stream<Error = io::Error>,
              S::Item: ReadFrom<Self>
    {
        ReadStream(Phase::A((self, stream)))
    }
}
impl<R: Read> AsyncRead for R {}

/// A future to read and fold bytes stream.
#[derive(Debug)]
pub struct ReadFold<R, F, B, T>(Option<(ReadBytes<R, B>, T, F)>);
impl<R: Read, F, B, T> Future for ReadFold<R, F, B, T>
    where B: AsMut<[u8]>,
          F: Fn(T, B, usize) -> Result<(B, T), (B, io::Result<T>)>
{
    type Item = (R, B, T);
    type Error = (R, B, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut state = self.0.take().expect("Cannot poll ReadFold twice");
        if let Async::Ready((r, buf, size)) = state.0.poll()? {
            let (_, acc, fold) = state;
            match fold(acc, buf, size) {
                Ok((buf, acc)) => {
                    self.0 = Some((r.async_read(buf), acc, fold));
                    self.poll()
                }
                Err((buf, Ok(acc))) => Ok(Async::Ready((r, buf, acc))),
                Err((buf, Err(e))) => Err((r, buf, e)),
            }
        } else {
            self.0 = Some(state);
            Ok(Async::NotReady)
        }
    }
}

/// A future which will read non empty bytes from `R`.
#[derive(Debug)]
pub struct ReadNonEmpty<R, B>(ReadBytes<R, B>);
impl<R: Read, B: AsMut<[u8]>> Future for ReadNonEmpty<R, B> {
    type Item = (R, B, usize);
    type Error = (R, B, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((r, mut b, size)) = self.0.poll()? {
            if size == 0 && !b.as_mut().is_empty() {
                let e = io::Error::new(io::ErrorKind::UnexpectedEof,
                                       format!("Unexpected Eof ({} bytes are required)",
                                               b.as_mut().len()));
                Err((r, b, e))
            } else {
                Ok(Async::Ready((r, b, size)))
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}

/// A future which will read bytes from `R`.
#[derive(Debug)]
pub struct ReadBytes<R, B>(Option<(R, B)>);
impl<R: Read, B: AsMut<[u8]>> Future for ReadBytes<R, B> {
    type Item = (R, B, usize);
    type Error = (R, B, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (mut inner, mut buf) = self.0.take().expect("Cannot poll ReadBytes twice");
        match inner.read(buf.as_mut()) {
            Ok(size) => Ok(Async::Ready((inner, buf, size))),
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    self.0 = Some((inner, buf));
                    Ok(Async::NotReady)
                } else {
                    Err((inner, buf, e))
                }
            }
        }
    }
}

/// A future which will read bytes required to fill the buffer `B` completely.
#[derive(Debug)]
pub struct ReadExact<R, B>(ReadNonEmpty<R, Window<B>>);
impl<R, B> Future for ReadExact<R, B>
    where R: Read,
          B: AsMut<[u8]>
{
    type Item = (R, B);
    type Error = (R, B, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((r, b, read_size)) = self.0
            .poll()
            .map_err(|(r, b, e)| (r, b.into_inner(), e))? {
            let mut b = b.skip(read_size);
            if b.as_mut().is_empty() {
                Ok(Async::Ready((r, b.into_inner())))
            } else {
                self.0 = r.async_read_non_empty(b);
                self.poll()
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}

/// A stream which will read values associated the patterns generated from `S`.
pub struct ReadStream<R, S>(Phase<(R, S), (<S::Item as ReadFrom<R>>::Future, S)>)
    where R: Read,
          S: futures::Stream,
          S::Item: ReadFrom<R>;
impl<R: Read, S> futures::Stream for ReadStream<R, S>
    where S: futures::Stream<Error = io::Error>,
          S::Item: ReadFrom<R>
{
    type Item = <S::Item as Pattern>::Value;
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.0.take() {
            Phase::A((r, mut s)) => {
                match s.poll() {
                    Err(e) => Err((r, e)),
                    Ok(Async::NotReady) => {
                        self.0 = Phase::A((r, s));
                        Ok(Async::NotReady)
                    }
                    Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
                    Ok(Async::Ready(Some(p))) => {
                        self.0 = Phase::B((p.lossless_read_from(r), s));
                        self.poll()
                    }
                }
            }
            Phase::B((mut f, s)) => {
                if let Async::Ready((r, v)) = f.poll()? {
                    self.0 = Phase::A((r, s));
                    Ok(Async::Ready(Some(v)))
                } else {
                    self.0 = Phase::B((f, s));
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll ReadStream which has been finished"),
        }
    }
}
