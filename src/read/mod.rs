use std::io::{self, Read};
use std::mem;
use futures::{Poll, Async, Future, Stream};

use super::IoFuture;
use pattern::{Pattern, Window};

pub mod primitives;
pub mod combinators;

pub trait ReadFrom<R: Read>: Pattern {
    type Future: Future<Item = (R, Self::Value), Error = (R, io::Error)>;

    fn read_from(self, reader: R) -> Self::Future;

    fn sync_read_from(self, reader: R) -> io::Result<Self::Value> {
        self.read_from(reader).map(|(_, v)| v).map_err(|(_, e)| e).wait()
    }
    fn boxed(self) -> BoxReadFrom<R, Self::Value>
        where Self: Send + 'static,
              Self::Future: Send + 'static
    {
        let mut f = Some(move |reader: R| self.read_from(reader).boxed());
        BoxReadFrom(Box::new(move |reader| (f.take().unwrap())(reader)))
    }
}

pub struct BoxReadFrom<R, T>(Box<FnMut(R) -> IoFuture<R, T>>);
impl<R, T> Pattern for BoxReadFrom<R, T> {
    type Value = T;
}
impl<R: Read, T> ReadFrom<R> for BoxReadFrom<R, T> {
    type Future = IoFuture<R, T>;
    fn read_from(mut self, reader: R) -> Self::Future {
        (self.0)(reader)
    }
}

pub trait AsyncRead: Read + Sized {
    fn async_read<B: AsMut<[u8]>>(self, buf: B) -> ReadBytes<Self, B> {
        ReadBytes::State(self, buf)
    }
    fn async_read_non_empty<B: AsMut<[u8]>>(self, buf: B) -> ReadNonEmpty<Self, B> {
        ReadNonEmpty(self.async_read(buf))
    }
    fn async_read_exact<B: AsMut<[u8]>>(self, buf: B) -> ReadExact<Self, B> {
        ReadExact(self.async_read_non_empty(Window::new_mut(buf)))
    }
    fn async_read_fold<F, B, T>(self, buf: B, init: T, f: F) -> ReadFold<Self, F, B, T>
        where F: Fn(T, B, usize) -> Result<(B, T), (B, io::Result<T>)>,
              B: AsMut<[u8]>
    {
        ReadFold(Some((self.async_read(buf), init, f)))
    }
    fn into_bytes_stream(self, buffer_size: usize) -> BytesStream<Self> {
        BytesStream(self.async_read(vec![0; buffer_size]))
    }
}
impl<R: Read> AsyncRead for R {}

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

#[derive(Debug)]
pub struct BytesStream<R>(ReadBytes<R, Vec<u8>>);
impl<R: Read> Stream for BytesStream<R> {
    type Item = Vec<u8>;
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if let Async::Ready((r, b, size)) = self.0.poll().map_err(|(r, _, e)| (r, e))? {
            if size == 0 {
                Ok(Async::Ready(None))
            } else {
                let bytes = Vec::from(&b[0..b.len()]);
                self.0 = r.async_read(b);
                Ok(Async::Ready(Some(bytes)))
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}
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

#[derive(Debug)]
pub enum ReadBytes<R, B> {
    State(R, B),
    Polled,
}
impl<R: Read, B: AsMut<[u8]>> Future for ReadBytes<R, B> {
    type Item = (R, B, usize);
    type Error = (R, B, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match mem::replace(self, ReadBytes::Polled) {
            ReadBytes::State(mut inner, mut buf) => {
                match inner.read(buf.as_mut()) {
                    Ok(size) => Ok(Async::Ready((inner, buf, size))),
                    Err(e) => {
                        if e.kind() == io::ErrorKind::WouldBlock {
                            *self = ReadBytes::State(inner, buf);
                            Ok(Async::NotReady)
                        } else {
                            Err((inner, buf, e))
                        }
                    }
                }
            }
            ReadBytes::Polled => panic!("Cannot poll Read twice"),
        }
    }
}

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
