use std::io::{self, Write};
use futures::{Poll, Async, Future};

use pattern::{Pattern, Window};

pub trait WriteTo<W: Write>: Pattern {
    type Future: Future<Item = (W, Self::Value), Error = (W, io::Error)>;

    fn write_to(self, writer: W) -> Self::Future;

    fn sync_write_to(self, writer: W) -> io::Result<Self::Value> {
        self.write_to(writer).map(|(_, v)| v).map_err(|(_, e)| e).wait()
    }
    // fn boxed(self) -> BoxWriteTo<W, Self::Value>
    //     where Self: Send + 'static,
    //           Self::Future: Send + 'static
    // {
    //     let mut f = Some(move |writer: W| self.write_to(writer).boxed());
    //     BoxWriteTo(Box::new(move |writer| (f.take().unwrap())(writer)))
    // }
}

pub trait AsyncWrite: Write + Sized {
    fn async_write<B: AsRef<[u8]>>(self, buf: B) -> WriteBytes<Self, B> {
        WriteBytes(Some((self, buf)))
    }
    fn async_write_all<B: AsRef<[u8]>>(self, buf: B) -> WriteAll<Self, B> {
        WriteAll(self.async_write(Window::new(buf)))
    }
    fn async_flush(self) -> Flush<Self> {
        Flush(Some(self))
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
