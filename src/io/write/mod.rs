use std::io::{self, Write};
use futures::{self, Poll, Async, Future};

use pattern::{Pattern, Window};
use super::futures::IoFuture;
use super::common::Phase;

pub mod primitives;
pub mod combinators;

pub trait WriteTo<W: Write>: Pattern {
    type Future: Future<Item = (W, Self::Value), Error = (W, io::Error)>;

    fn lossless_write_to(self, writer: W) -> Self::Future;

    fn write_to(self, writer: W) -> LossyWriteTo<W, Self::Future> {
        fn conv<W>((_, e): (W, io::Error)) -> io::Error {
            e
        };
        self.lossless_write_to(writer).map_err(conv as _)
    }
    fn sync_write_to(self, writer: W) -> io::Result<Self::Value> {
        self.lossless_write_to(writer).map(|(_, v)| v).map_err(|(_, e)| e).wait()
    }
    fn boxed(self) -> BoxWriteTo<W, Self::Value>
        where Self: Send + 'static,
              Self::Future: Send + 'static
    {
        let mut f = Some(move |writer: W| self.lossless_write_to(writer).boxed());
        BoxWriteTo(Box::new(move |writer| (f.take().unwrap())(writer)))
    }
}

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

pub trait AsyncWrite: Write + Sized {
    fn async_write<B: AsRef<[u8]>>(self, buf: B) -> WriteBytes<Self, B> {
        WriteBytes(Some((self, buf)))
    }
    fn async_write_all<B: AsRef<[u8]>>(self, buf: B) -> WriteAll<Self, B> {
        WriteAll(self.async_write(Window::new_ref(buf)))
    }
    fn async_flush(self) -> Flush<Self> {
        Flush(Some(self))
    }
    fn async_write_stream<S>(self, stream: S) -> WriteStream<Self, S>
        where S: futures::Stream<Error = io::Error>,
              S::Item: WriteTo<Self>
    {
        WriteStream(Phase::A((self, stream)))
    }
}
impl<W: Write> AsyncWrite for W {}

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
