use std::io::{self, Write};
use futures::{self, Poll, Async, Future};

use pattern::{Pattern, Window};
use super::futures::IoFuture;
use super::common::Phase;

pub mod primitives;
pub mod combinators;

/// The `WriteTo` trait allows for writing
/// a value of the pattern to a sink asynchronously.
///
/// # Notice
///
/// For executing asynchronously, we assume the writer `W`
/// returns the `std::io::ErrorKind::WouldBlock` error
/// if a write operation would be about to block.
pub trait WriteTo<W: Write>: Pattern {
    /// The future to write a value of the pattern to `W`.
    type Future: Future<Item = (W, Self::Value), Error = (W, io::Error)>;

    /// Creates a future instance to write a value of the pattern to `writer`.
    fn lossless_write_to(self, writer: W) -> Self::Future;

    /// Creates a future instance to write a value of the pattern to `writer`.
    ///
    /// If the execution of the future fails, the `writer` will be dropped.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate futures;
    /// extern crate handy_io;
    ///
    /// use futures::Future;
    /// use handy_io::io::WriteTo;
    /// use handy_io::pattern::{Pattern, Endian};
    ///
    /// fn main() {
    ///     let pattern = (1u8, 2u16.be());
    ///     let future = pattern.write_to(Vec::new());
    ///     assert_eq!(future.wait().unwrap().0, [1, 0, 2]);
    /// }
    /// ```
    fn write_to(self, writer: W) -> LossyWriteTo<W, Self::Future> {
        fn conv<W>((_, e): (W, io::Error)) -> io::Error {
            e
        };
        self.lossless_write_to(writer).map_err(conv as _)
    }

    /// Scynchronously writing a value of the pattern to `writer`.
    ///
    /// # Example
    ///
    /// ```
    /// use handy_io::io::WriteTo;
    /// use handy_io::pattern::{Pattern, Endian};
    ///
    /// let mut buf = [0; 3];
    /// let pattern = (1u8, 2u16.be());
    /// pattern.sync_write_to(&mut &mut buf[..]).unwrap();
    /// assert_eq!(buf, [1, 0, 2]);
    /// ```
    fn sync_write_to(self, writer: W) -> io::Result<Self::Value> {
        self.lossless_write_to(writer).map(|(_, v)| v).map_err(|(_, e)| e).wait()
    }

    /// Returns the boxed version of this pattern.
    fn boxed(self) -> BoxWriteTo<W, Self::Value>
        where Self: Send + 'static,
              Self::Future: Send + 'static
    {
        let mut f = Some(move |writer: W| self.lossless_write_to(writer).boxed());
        BoxWriteTo(Box::new(move |writer| (f.take().unwrap())(writer)))
    }
}

/// The lossy version of the `WriteTo` future `F`.
///
/// This future will drop `W`, if it's execution fails.
pub type LossyWriteTo<W, F> = futures::MapErr<F, fn((W, io::Error)) -> io::Error>;

/// Boxed object which implements `WriteTo` trait.
///
/// This object can be created with the `WriteTo::boxed` method.
pub struct BoxWriteTo<W, T>(Box<FnMut(W) -> IoFuture<W, T>>);
impl<W, T> Pattern for BoxWriteTo<W, T> {
    type Value = T;
}
impl<W: Write, T> WriteTo<W> for BoxWriteTo<W, T> {
    type Future = IoFuture<W, T>;
    fn lossless_write_to(mut self, writer: W) -> Self::Future {
        (self.0)(writer)
    }
}

/// An asynchronous version of the standard `Write` trait.
///
/// # Notice
///
/// For executing asynchronously, we assume the writer which implements this trait
/// returns the `std::io::ErrorKind::WouldBlock` error
/// if a write operation would be about to block.
pub trait AsyncWrite: Write + Sized {
    /// Creates a future which will write bytes asynchronously.
    fn async_write<B: AsRef<[u8]>>(self, buf: B) -> WriteBytes<Self, B> {
        WriteBytes(Some((self, buf)))
    }

    /// Creates a future which will write all bytes asynchronously.
    fn async_write_all<B: AsRef<[u8]>>(self, buf: B) -> WriteAll<Self, B> {
        WriteAll(self.async_write(Window::new_ref(buf)))
    }

    /// Creates a future which will flush the internal buffer.
    fn async_flush(self) -> Flush<Self> {
        Flush(Some(self))
    }

    /// Creates a stream which will write values associated
    /// the patterns generated from `stream`.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate handy_io;
    /// extern crate futures;
    ///
    /// use handy_io::io::AsyncWrite;
    /// use futures::{Async, Stream};
    ///
    /// fn main() {
    ///     let mut writer = [0; 2];
    ///     {
    ///         let patterns = futures::stream::iter(vec![Ok(1u8), Ok(2u8)].into_iter());
    ///         let mut stream = writer.async_write_stream(patterns);
    ///
    ///         assert_eq!(stream.poll().unwrap(), Async::Ready(Some(())));
    ///         assert_eq!(stream.poll().unwrap(), Async::Ready(Some(())));
    ///         assert_eq!(stream.poll().unwrap(), Async::Ready(None));
    ///     }
    ///     assert_eq!(writer, [1, 2]);
    /// }
    /// ```
    fn async_write_stream<S>(self, stream: S) -> WriteStream<Self, S>
        where S: futures::Stream<Error = io::Error>,
              S::Item: WriteTo<Self>
    {
        WriteStream(Phase::A((self, stream)))
    }
}
impl<W: Write> AsyncWrite for W {}

/// A future which will write bytes to `W`.
#[derive(Debug)]
pub struct WriteBytes<W, B>(Option<(W, B)>);
impl<W: Write, B: AsRef<[u8]>> Future for WriteBytes<W, B> {
    type Item = (W, B, usize);
    type Error = (W, B, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (mut w, b) = self.0.take().expect("Cannot poll WriteBytes twice");
        match w.write(b.as_ref()) {
            Ok(write_size) => Ok(Async::Ready((w, b, write_size))),
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    self.0 = Some((w, b));
                    Ok(Async::NotReady)
                } else {
                    Err((w, b, e))
                }
            }
        }
    }
}

/// A future which will write all bytes to `W`.
#[derive(Debug)]
pub struct WriteAll<W, B>(WriteBytes<W, Window<B>>);
impl<W: Write, B: AsRef<[u8]>> Future for WriteAll<W, B> {
    type Item = (W, B);
    type Error = (W, B, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((w, b, size)) = self.0
            .poll()
            .map_err(|(w, b, e)| (w, b.into_inner(), e))? {
            let b = b.skip(size);
            if b.as_ref().is_empty() {
                Ok(Async::Ready((w, b.into_inner())))
            } else {
                self.0 = w.async_write(b);
                self.poll()
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}

/// A future which will flush the internal buffer of `W`.
#[derive(Debug)]
pub struct Flush<W>(Option<W>);
impl<W: Write> Future for Flush<W> {
    type Item = W;
    type Error = (W, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut w = self.0.take().expect("Cannot poll Flush twice");
        match w.flush() {
            Ok(()) => Ok(Async::Ready(w)),
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    self.0 = Some(w);
                    Ok(Async::NotReady)
                } else {
                    Err((w, e))
                }
            }
        }
    }
}

/// A stream which will write values associated the patterns generated from `S`.
pub struct WriteStream<W, S>(Phase<(W, S), (<S::Item as WriteTo<W>>::Future, S)>)
    where W: Write,
          S: futures::Stream,
          S::Item: WriteTo<W>;
impl<W: Write, S> futures::Stream for WriteStream<W, S>
    where S: futures::Stream<Error = io::Error>,
          S::Item: WriteTo<W>
{
    type Item = <S::Item as Pattern>::Value;
    type Error = (W, io::Error);
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.0.take() {
            Phase::A((w, mut s)) => {
                match s.poll() {
                    Err(e) => Err((w, e)),
                    Ok(Async::NotReady) => {
                        self.0 = Phase::A((w, s));
                        Ok(Async::NotReady)
                    }
                    Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
                    Ok(Async::Ready(Some(p))) => {
                        self.0 = Phase::B((p.lossless_write_to(w), s));
                        self.poll()
                    }
                }
            }
            Phase::B((mut f, s)) => {
                if let Async::Ready((w, v)) = f.poll()? {
                    self.0 = Phase::A((w, s));
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
