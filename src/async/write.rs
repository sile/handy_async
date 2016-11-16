use std::io;
use std::marker::PhantomData;
use futures;
use futures::Poll;
use futures::Async;
use futures::Future;

use pattern::write::{LE, BE, U24, I24, Iter};
use super::Window;
use super::IoFuture;

pub trait AsyncWrite: Sized + io::Write {
    fn async_write<T>(self, buf: T) -> Write<Self, T>
        where T: AsRef<[u8]>
    {
        Write(Some((self, buf)))
    }

    fn async_write_all<T>(self, buf: T) -> WriteAll<Self, T>
        where T: AsRef<[u8]>
    {
        WriteAll(self.async_write(Window::new(buf)))
    }
    fn async_write_pattern<P>(self, pattern: P) -> P::Future
        where P: WritePattern<Self>
    {
        pattern.write_pattern(self)
    }
}
impl<T> AsyncWrite for T where T: io::Write {}

pub struct Write<W, B>(Option<(W, B)>);
impl<W, B> Future for Write<W, B>
    where W: io::Write,
          B: AsRef<[u8]>
{
    type Item = (W, B, usize);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (mut writer, buf) = self.0.take().expect("Cannot poll Write twice");
        match writer.write(buf.as_ref()) {
            Ok(write_size) => Ok(Async::Ready((writer, buf, write_size))),
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    self.0 = Some((writer, buf));
                    Ok(Async::NotReady)
                } else {
                    Err(e)
                }
            }
        }
    }
}

pub struct WriteAll<W, B>(Write<W, Window<B>>);
impl<W, B> Future for WriteAll<W, B>
    where W: io::Write,
          B: AsRef<[u8]>
{
    type Item = (W, B);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((writer, mut buf, write_size)) = self.0.poll()? {
            buf.offset += write_size;
            if buf.offset == buf.inner.as_ref().len() {
                Ok(Async::Ready((writer, buf.inner)))
            } else {
                self.0 = writer.async_write(buf);
                Ok(Async::NotReady)
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}

pub trait WritePattern<W> {
    type Output;
    type Future: Future<Item = (W, Self::Output), Error = io::Error>;
    fn write_pattern(self, writer: W) -> Self::Future;

    fn boxed(self) -> BoxPattern<W, Self::Output>
        where Self: Sized + Send + 'static,
              Self::Future: Send + 'static
    {
        let mut f = Some(move |writer| self.write_pattern(writer).boxed());
        BoxPattern(Box::new(move |writer| (f.take().unwrap())(writer)))
    }
    fn map<F, T>(self, f: F) -> MapPattern<W, Self, F>
        where Self: Sized,
              F: FnOnce(Self::Output) -> T + Send + 'static
    {
        MapPattern(self, f, PhantomData)
    }
}

pub struct BoxPattern<W, O>(Box<FnMut(W) -> IoFuture<(W, O)> + Send + 'static>);
impl<W: io::Write, O> WritePattern<W> for BoxPattern<W, O> {
    type Output = O;
    type Future = IoFuture<(W, O)>;
    fn write_pattern(mut self, writer: W) -> Self::Future {
        (self.0)(writer)
    }
}

pub struct MapPattern<W, P, F>(P, F, PhantomData<W>);
impl<W: io::Write, P, F, U> WritePattern<W> for MapPattern<W, P, F>
    where P: WritePattern<W>,
          P::Future: Send + 'static,
          F: FnOnce(P::Output) -> U + Send + 'static
{
    type Output = U;
    type Future = IoFuture<(W, U)>;
    fn write_pattern(self, writer: W) -> Self::Future {
        let MapPattern(inner, map, _) = self;
        inner.write_pattern(writer).map(move |(w, v)| (w, map(v))).boxed()
    }
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
    ($p:ty, $b:expr) => {
        impl<W: io::Write> WritePattern<W> for $p {
            type Output = ();
            type Future = WriteFixed<W, [u8; $b]>;
            fn write_pattern(self, writer: W) -> Self::Future {
                use pattern::write::Fixed;
                let mut buf = [0; $b];
                self.write(&mut buf);
                WriteFixed(writer.async_write_all(buf))
            }
        }
    }
}

impl_fixed_write_pattern!(u8, 1);
impl_fixed_write_pattern!(LE<u16>, 2);
impl_fixed_write_pattern!(BE<u16>, 2);
impl_fixed_write_pattern!(LE<U24>, 3);
impl_fixed_write_pattern!(BE<U24>, 3);
impl_fixed_write_pattern!(LE<u32>, 4);
impl_fixed_write_pattern!(BE<u32>, 4);
impl_fixed_write_pattern!(LE<u64>, 8);
impl_fixed_write_pattern!(BE<u64>, 8);
impl_fixed_write_pattern!(i8, 1);
impl_fixed_write_pattern!(LE<i16>, 2);
impl_fixed_write_pattern!(BE<i16>, 2);
impl_fixed_write_pattern!(LE<I24>, 3);
impl_fixed_write_pattern!(BE<I24>, 3);
impl_fixed_write_pattern!(LE<i32>, 4);
impl_fixed_write_pattern!(BE<i32>, 4);
impl_fixed_write_pattern!(LE<i64>, 8);
impl_fixed_write_pattern!(BE<i64>, 8);

impl<W: io::Write> WritePattern<W> for String {
    type Output = Self;
    type Future = WriteAll<W, Self>;
    fn write_pattern(self, writer: W) -> Self::Future {
        writer.async_write_all(self)
    }
}

impl<W: io::Write> WritePattern<W> for () {
    type Output = ();
    type Future = futures::Finished<(W, Self::Output), io::Error>;
    fn write_pattern(self, writer: W) -> Self::Future {
        futures::finished((writer, ()))
    }
}

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

impl<W: io::Write, P0, P1, P2, P3, P4, P5> WritePattern<W> for (P0, P1, P2, P3, P4, P5)
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
          P4::Future: Send + 'static,
          P5: WritePattern<W> + Send + 'static,
          P5::Output: Send + 'static,
          P5::Future: Send + 'static
{
    type Output = (P0::Output, P1::Output, P2::Output, P3::Output, P4::Output, P5::Output);
    type Future = IoFuture<(W, Self::Output)>;
    fn write_pattern(self, writer: W) -> Self::Future {
        let (p0, p1, p2, p3, p4, p5) = self;
        p0.write_pattern(writer)
            .and_then(move |(w, o0)| {
                p1.write_pattern(w).and_then(move |(w, o1)| {
                    p2.write_pattern(w).and_then(move |(w, o2)| {
                        p3.write_pattern(w).and_then(move |(w, o3)| {
                            p4.write_pattern(w).and_then(move |(w, o4)| {
                                p5.write_pattern(w).map(|(w, o5)| (w, (o0, o1, o2, o3, o4, o5)))
                            })
                        })
                    })
                })
            })
            .boxed()
    }
}

impl<W: io::Write, I, P> WritePattern<W> for Iter<I>
    where I: Iterator<Item = P>,
          P: WritePattern<W>
{
    type Output = ();
    type Future = WriteIter<W, I, P::Future>;
    fn write_pattern(self, writer: W) -> Self::Future {
        let mut iter = self.0;
        if let Some(p) = iter.next() {
            WriteIter(iter, Ok(p.write_pattern(writer)))
        } else {
            WriteIter(iter, Err(Some(writer)))
        }
    }
}

#[derive(Debug)]
pub struct WriteIter<W, I, F>(I, Result<F, Option<W>>);
impl<I, P, W, F> Future for WriteIter<W, I, F>
    where I: Iterator<Item = P>,
          P: WritePattern<W, Future = F>,
          F: Future<Item = (W, P::Output), Error = io::Error>,
          W: io::Write
{
    type Item = (W, ());
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use std::mem;
        match mem::replace(&mut self.1, Err(None)) {
            Ok(mut f) => {
                if let Async::Ready((w, _)) = f.poll()? {
                    if let Some(p) = self.0.next() {
                        self.1 = Ok(p.write_pattern(w));
                    } else {
                        self.1 = Err(Some(w));
                    }
                    self.poll()
                } else {
                    self.1 = Ok(f);
                    Ok(Async::NotReady)
                }
            }
            Err(w) => {
                if let Some(w) = w {
                    Ok(Async::Ready((w, ())))
                } else {
                    panic!("Cannot poll WriteIter twice");
                }
            }
        }
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
        let output = output.async_write_pattern((1u8, BE(2u16), LE(3u32)).map(|_| ()).boxed())
            .wait()
            .unwrap()
            .0;
        assert_eq!(output, [1, 0, 2, 3, 0, 0, 0]);
    }
}
