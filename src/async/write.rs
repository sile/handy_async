use std::io;
use tokio_core;
use tokio_core::io::WriteAll;
use tokio_core::io::IoFuture;
use futures::Poll;
use futures::Async;
use futures::Future;

use pattern;

pub trait AsyncWrite: Sized + io::Write {
    fn async_write_all<T>(self, buf: T) -> WriteAll<Self, T>
        where T: AsRef<[u8]>
    {
        tokio_core::io::write_all(self, buf)
    }
    fn async_write_pattern<P>(self, pattern: P) -> P::Future
        where P: WritePattern<Self>
    {
        pattern.write_pattern(self)
    }
}
impl<T> AsyncWrite for T where T: io::Write {}

pub trait WritePattern<W> {
    type Output;
    type Future: Future<Item = (W, Self::Output), Error = io::Error>;
    fn write_pattern(self, writer: W) -> Self::Future;
}

pub struct WriteFixed<W, B>(WriteAll<W, B>);
impl<W: io::Write, B: AsRef<[u8]>> Future for WriteFixed<W, B> {
    type Item = (W, ());
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((writer, _)) = self.0.poll()? {
            Ok(Async::Ready((writer, ())))
        } else {
            Ok(Async::NotReady)
        }
    }
}

macro_rules! impl_fixed_write_pattern {
    ($p:ident, $b:expr) => {
        impl<W: io::Write> WritePattern<W> for pattern::write::$p {
            type Output = ();
            type Future = WriteFixed<W, [u8; $b]>;
            fn write_pattern(self, writer: W) -> Self::Future {
                let mut buf = [0; $b];
                self.write(&mut buf);
                WriteFixed(writer.async_write_all(buf))
            }
        }
    }
}
impl_fixed_write_pattern!(U8, 1);
impl_fixed_write_pattern!(U16le, 2);
impl_fixed_write_pattern!(U16be, 2);
impl_fixed_write_pattern!(U24le, 3);
impl_fixed_write_pattern!(U24be, 3);
impl_fixed_write_pattern!(U32le, 4);
impl_fixed_write_pattern!(U32be, 4);
impl_fixed_write_pattern!(U64le, 8);
impl_fixed_write_pattern!(U64be, 8);
impl_fixed_write_pattern!(I8, 1);
impl_fixed_write_pattern!(I16le, 2);
impl_fixed_write_pattern!(I16be, 2);
impl_fixed_write_pattern!(I24le, 3);
impl_fixed_write_pattern!(I24be, 3);
impl_fixed_write_pattern!(I32le, 4);
impl_fixed_write_pattern!(I32be, 4);
impl_fixed_write_pattern!(I64le, 8);
impl_fixed_write_pattern!(I64be, 8);

impl<W: io::Write, P0, P1> WritePattern<W> for (P0, P1)
    where P0: WritePattern<W> + Send + 'static,
          P0::Output: Send + 'static,
          P0::Future: Send + 'static,
          P1: WritePattern<W> + Send + 'static,
          P1::Output: Send + 'static,
          P1::Future: Send + 'static
{
    type Output = (P0::Output, P1::Output);
    type Future = IoFuture<(W, Self::Output)>;
    fn write_pattern(self, writer: W) -> Self::Future {
        let (p0, p1) = self;
        p0.write_pattern(writer)
            .and_then(move |(w, o0)| p1.write_pattern(w).map(|(w, o1)| (w, (o0, o1))))
            .boxed()
    }
}

impl<W: io::Write, P0, P1, P2> WritePattern<W> for (P0, P1, P2)
    where P0: WritePattern<W> + Send + 'static,
          P0::Output: Send + 'static,
          P0::Future: Send + 'static,
          P1: WritePattern<W> + Send + 'static,
          P1::Output: Send + 'static,
          P1::Future: Send + 'static,
          P2: WritePattern<W> + Send + 'static,
          P2::Output: Send + 'static,
          P2::Future: Send + 'static
{
    type Output = (P0::Output, P1::Output, P2::Output);
    type Future = IoFuture<(W, Self::Output)>;
    fn write_pattern(self, writer: W) -> Self::Future {
        let (p0, p1, p2) = self;
        p0.write_pattern(writer)
            .and_then(move |(w, o0)| {
                p1.write_pattern(w).and_then(move |(w, o1)| {
                    p2.write_pattern(w).map(|(w, o2)| (w, (o0, o1, o2)))
                })
            })
            .boxed()
    }
}

impl<W: io::Write, P0, P1, P2, P3> WritePattern<W> for (P0, P1, P2, P3)
    where P0: WritePattern<W> + Send + 'static,
          P0::Output: Send + 'static,
          P0::Future: Send + 'static,
          P1: WritePattern<W> + Send + 'static,
          P1::Output: Send + 'static,
          P1::Future: Send + 'static,
          P2: WritePattern<W> + Send + 'static,
          P2::Output: Send + 'static,
          P2::Future: Send + 'static,
          P3: WritePattern<W> + Send + 'static,
          P3::Output: Send + 'static,
          P3::Future: Send + 'static
{
    type Output = (P0::Output, P1::Output, P2::Output, P3::Output);
    type Future = IoFuture<(W, Self::Output)>;
    fn write_pattern(self, writer: W) -> Self::Future {
        let (p0, p1, p2, p3) = self;
        p0.write_pattern(writer)
            .and_then(move |(w, o0)| {
                p1.write_pattern(w).and_then(move |(w, o1)| {
                    p2.write_pattern(w).and_then(move |(w, o2)| {
                        p3.write_pattern(w).map(|(w, o3)| (w, (o0, o1, o2, o3)))
                    })
                })
            })
            .boxed()
    }
}

impl<W: io::Write, P0, P1, P2, P3, P4> WritePattern<W> for (P0, P1, P2, P3, P4)
    where P0: WritePattern<W> + Send + 'static,
          P0::Output: Send + 'static,
          P0::Future: Send + 'static,
          P1: WritePattern<W> + Send + 'static,
          P1::Output: Send + 'static,
          P1::Future: Send + 'static,
          P2: WritePattern<W> + Send + 'static,
          P2::Output: Send + 'static,
          P2::Future: Send + 'static,
          P3: WritePattern<W> + Send + 'static,
          P3::Output: Send + 'static,
          P3::Future: Send + 'static,
          P4: WritePattern<W> + Send + 'static,
          P4::Output: Send + 'static,
          P4::Future: Send + 'static
{
    type Output = (P0::Output, P1::Output, P2::Output, P3::Output, P4::Output);
    type Future = IoFuture<(W, Self::Output)>;
    fn write_pattern(self, writer: W) -> Self::Future {
        let (p0, p1, p2, p3, p4) = self;
        p0.write_pattern(writer)
            .and_then(move |(w, o0)| {
                p1.write_pattern(w).and_then(move |(w, o1)| {
                    p2.write_pattern(w).and_then(move |(w, o2)| {
                        p3.write_pattern(w).and_then(move |(w, o3)| {
                            p4.write_pattern(w).map(|(w, o4)| (w, (o0, o1, o2, o3, o4)))
                        })
                    })
                })
            })
            .boxed()
    }
}

#[cfg(test)]
mod test {
    use futures::Future;
    use pattern::write::*;
    use super::*;

    #[test]
    fn it_works() {
        let output = Vec::new();
        let output = output.async_write_pattern((U8(1), U16be(2), U32le(3))).wait().unwrap().0;
        assert_eq!(output, [1, 0, 2, 3, 0, 0, 0]);
    }
}
