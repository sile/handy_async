use std::io;
use std::marker::PhantomData;
use futures::Poll;
use futures::Async;
use futures::Future;

use pattern;
use super::Window;
use super::IoFuture;

const READ_TO_END_BUF_SIZE: usize = 1024;

pub trait AsyncRead: Sized + io::Read {
    fn async_read<T>(self, buf: T) -> Read<Self, T>
        where T: AsMut<[u8]>
    {
        Read::new(self, buf)
    }
    fn async_read_exact<T>(self, buf: T) -> ReadExact<Self, T>
        where T: AsMut<[u8]>
    {
        ReadExact(self.async_read(Window::new(buf)))
    }
    fn async_read_to_end(self, mut buf: Vec<u8>) -> ReadToEnd<Self> {
        let offset = buf.len();
        buf.resize(offset + READ_TO_END_BUF_SIZE, 0);
        ReadToEnd(self.async_read(Window::with_offset(buf, offset)))
    }
    fn async_read_pattern<P>(self, pattern: P) -> P::Future
        where P: ReadPattern<Self>
    {
        pattern.read_pattern(self)
    }
}
impl<T> AsyncRead for T where T: Sized + io::Read {}

pub struct Read<R, B> {
    inner: Option<(R, B)>,
}
impl<R, B> Read<R, B>
    where R: io::Read,
          B: AsMut<[u8]>
{
    fn new(reader: R, buf: B) -> Self {
        Read { inner: Some((reader, buf)) }
    }
}
impl<R, B> Future for Read<R, B>
    where R: io::Read,
          B: AsMut<[u8]>
{
    type Item = (R, B, usize);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (mut reader, mut buf) = self.inner.take().expect("Cannot poll Read twice");
        match reader.read(buf.as_mut()) {
            Ok(read_size) => Ok(Async::Ready((reader, buf, read_size))),
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    self.inner = Some((reader, buf));
                    Ok(Async::NotReady)
                } else {
                    Err(e)
                }
            }
        }
    }
}

pub struct ReadExact<R, B>(Read<R, Window<B>>);
impl<R, B> Future for ReadExact<R, B>
    where R: io::Read,
          B: AsMut<[u8]>
{
    type Item = (R, B);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((reader, mut buf, read_size)) = self.0.poll()? {
            if read_size == 0 {
                let e = io::Error::new(io::ErrorKind::UnexpectedEof, "Unexpected EOF in ReadExact");
                Err(e)
            } else {
                buf.offset += read_size;
                if buf.offset == buf.inner.as_mut().len() {
                    Ok(Async::Ready((reader, buf.inner)))
                } else {
                    self.0 = reader.async_read(buf);
                    Ok(Async::NotReady)
                }
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}

pub struct ReadToEnd<R>(Read<R, Window<Vec<u8>>>);
impl<R> Future for ReadToEnd<R>
    where R: io::Read
{
    type Item = (R, Vec<u8>);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((reader, mut buf, read_size)) = self.0.poll()? {
            if read_size == 0 {
                buf.inner.truncate(buf.offset);
                Ok(Async::Ready((reader, buf.inner)))
            } else {
                buf.offset += read_size;
                if buf.offset == buf.inner.len() {
                    let new_len = buf.inner.len() + READ_TO_END_BUF_SIZE;
                    buf.inner.resize(new_len, 0);
                }
                self.0 = reader.async_read(buf);
                Ok(Async::NotReady)
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}

pub trait ReadPattern<R: io::Read> {
    type Output;
    type Future: Future<Item = (R, Self::Output), Error = io::Error>;
    fn read_pattern(self, reader: R) -> Self::Future;
}

pub struct ReadFixed<R, B, P> {
    read: ReadExact<R, B>,
    _pattern: PhantomData<P>,
}
impl<R: io::Read, B: AsMut<[u8]>, P: pattern::read::Fixed> ReadFixed<R, B, P> {
    pub fn new(read: ReadExact<R, B>) -> Self {
        ReadFixed {
            read: read,
            _pattern: PhantomData,
        }
    }
}
impl<R: io::Read, B: AsMut<[u8]>, P: pattern::read::Fixed> Future for ReadFixed<R, B, P> {
    type Item = (R, P::Output);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((r, mut b)) = self.read.poll()? {
            Ok(Async::Ready((r, P::convert(b.as_mut()))))
        } else {
            Ok(Async::NotReady)
        }
    }
}

macro_rules! impl_fixed_read_pattern {
    ($p:ident, $b:expr) => {
        impl<R: io::Read> ReadPattern<R> for pattern::read::$p {
            type Output = <pattern::read::$p as pattern::read::Fixed>::Output;
            type Future = ReadFixed<R, [u8; $b], Self>;
            fn read_pattern(self, reader: R) -> Self::Future {
                ReadFixed::new(reader.async_read_exact([0; $b]))
            }
        }
    }
}
impl_fixed_read_pattern!(U8, 1);
impl_fixed_read_pattern!(U16le, 2);
impl_fixed_read_pattern!(U16be, 2);
impl_fixed_read_pattern!(U24le, 3);
impl_fixed_read_pattern!(U24be, 3);
impl_fixed_read_pattern!(U32le, 4);
impl_fixed_read_pattern!(U32be, 4);
impl_fixed_read_pattern!(U64le, 8);
impl_fixed_read_pattern!(U64be, 8);
impl_fixed_read_pattern!(I8, 1);
impl_fixed_read_pattern!(I16le, 2);
impl_fixed_read_pattern!(I16be, 2);
impl_fixed_read_pattern!(I24le, 3);
impl_fixed_read_pattern!(I24be, 3);
impl_fixed_read_pattern!(I32le, 4);
impl_fixed_read_pattern!(I32be, 4);
impl_fixed_read_pattern!(I64le, 8);
impl_fixed_read_pattern!(I64be, 8);

impl<R: io::Read, P0, P1> ReadPattern<R> for (P0, P1)
    where P0: ReadPattern<R> + Send + 'static,
          P0::Output: Send + 'static,
          P0::Future: Send + 'static,
          P1: ReadPattern<R> + Send + 'static,
          P1::Output: Send + 'static,
          P1::Future: Send + 'static
{
    type Output = (P0::Output, P1::Output);
    type Future = IoFuture<(R, Self::Output)>;
    fn read_pattern(self, reader: R) -> Self::Future {
        let (p0, p1) = self;
        p0.read_pattern(reader)
            .and_then(move |(reader, o0)| {
                p1.read_pattern(reader).map(|(reader, o1)| (reader, (o0, o1)))
            })
            .boxed()
    }
}

impl<R: io::Read, P0, P1, P2> ReadPattern<R> for (P0, P1, P2)
    where P0: ReadPattern<R> + Send + 'static,
          P0::Output: Send + 'static,
          P0::Future: Send + 'static,
          P1: ReadPattern<R> + Send + 'static,
          P1::Output: Send + 'static,
          P1::Future: Send + 'static,
          P2: ReadPattern<R> + Send + 'static,
          P2::Output: Send + 'static,
          P2::Future: Send + 'static
{
    type Output = (P0::Output, P1::Output, P2::Output);
    type Future = IoFuture<(R, Self::Output)>;
    fn read_pattern(self, reader: R) -> Self::Future {
        let (p0, p1, p2) = self;
        p0.read_pattern(reader)
            .and_then(move |(reader, o0)| {
                p1.read_pattern(reader).and_then(move |(reader, o1)| {
                    p2.read_pattern(reader).map(|(reader, o2)| (reader, (o0, o1, o2)))
                })
            })
            .boxed()
    }
}

impl<R: io::Read, P0, P1, P2, P3> ReadPattern<R> for (P0, P1, P2, P3)
    where P0: ReadPattern<R> + Send + 'static,
          P0::Output: Send + 'static,
          P0::Future: Send + 'static,
          P1: ReadPattern<R> + Send + 'static,
          P1::Output: Send + 'static,
          P1::Future: Send + 'static,
          P2: ReadPattern<R> + Send + 'static,
          P2::Output: Send + 'static,
          P2::Future: Send + 'static,
          P3: ReadPattern<R> + Send + 'static,
          P3::Output: Send + 'static,
          P3::Future: Send + 'static
{
    type Output = (P0::Output, P1::Output, P2::Output, P3::Output);
    type Future = IoFuture<(R, Self::Output)>;
    fn read_pattern(self, reader: R) -> Self::Future {
        let (p0, p1, p2, p3) = self;
        p0.read_pattern(reader)
            .and_then(move |(reader, o0)| {
                p1.read_pattern(reader).and_then(move |(reader, o1)| {
                    p2.read_pattern(reader).and_then(move |(reader, o2)| {
                        p3.read_pattern(reader).map(|(reader, o3)| (reader, (o0, o1, o2, o3)))
                    })
                })
            })
            .boxed()
    }
}

impl<R: io::Read, P0, P1, P2, P3, P4> ReadPattern<R> for (P0, P1, P2, P3, P4)
    where P0: ReadPattern<R> + Send + 'static,
          P0::Output: Send + 'static,
          P0::Future: Send + 'static,
          P1: ReadPattern<R> + Send + 'static,
          P1::Output: Send + 'static,
          P1::Future: Send + 'static,
          P2: ReadPattern<R> + Send + 'static,
          P2::Output: Send + 'static,
          P2::Future: Send + 'static,
          P3: ReadPattern<R> + Send + 'static,
          P3::Output: Send + 'static,
          P3::Future: Send + 'static,
          P4: ReadPattern<R> + Send + 'static,
          P4::Output: Send + 'static,
          P4::Future: Send + 'static
{
    type Output = (P0::Output, P1::Output, P2::Output, P3::Output, P4::Output);
    type Future = IoFuture<(R, Self::Output)>;
    fn read_pattern(self, reader: R) -> Self::Future {
        let (p0, p1, p2, p3, p4) = self;
        p0.read_pattern(reader)
            .and_then(move |(reader, o0)| {
                p1.read_pattern(reader).and_then(move |(reader, o1)| {
                    p2.read_pattern(reader).and_then(move |(reader, o2)| {
                        p3.read_pattern(reader).and_then(move |(reader, o3)| {
                            p4.read_pattern(reader)
                                .map(|(reader, o4)| (reader, (o0, o1, o2, o3, o4)))
                        })
                    })
                })
            })
            .boxed()
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;
    use futures::Future;
    use pattern::read::*;
    use super::*;

    #[test]
    fn it_works() {
        let input = Cursor::new([0, 0, 1, 2, 0, 0, 0]);
        assert_eq!(input.async_read_pattern((U8, U16be, U32le)).wait().ok().map(|x| x.1),
                   Some((0, 1, 2)));
    }
}
