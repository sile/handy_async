use std::io;
use std::marker::PhantomData;
use futures;
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
    fn into_bytes_stream(self) -> BytesStream<Self> {
        BytesStream(self.async_read(vec![0; 1024]))
    }
}
impl<T> AsyncRead for T where T: Sized + io::Read {}

pub struct BytesStream<R>(Read<R, Vec<u8>>);
impl<R> futures::stream::Stream for BytesStream<R>
    where R: io::Read
{
    type Item = Vec<u8>;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if let Async::Ready((reader, mut buf, size)) = self.0.poll()? {
            if size == 0 {
                Ok(Async::Ready(None))
            } else {
                buf.truncate(size);
                self.0 = reader.async_read(vec![0; 1024]);
                Ok(Async::Ready(Some(buf)))
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}

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
            if read_size == 0 && buf.offset < buf.inner.as_mut().len() {
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

    fn boxed(self) -> BoxPattern<R, Self::Output>
        where Self: Sized + Send + 'static,
              Self::Future: Send + 'static
    {
        let mut f = Some(move |reader| self.read_pattern(reader).boxed());
        BoxPattern(Box::new(move |reader| (f.take().unwrap())(reader)))
    }
    fn map<F, T>(self, f: F) -> MapPattern<R, Self, F>
        where Self: Sized,
              F: FnOnce(Self::Output) -> T + Send + 'static
    {
        MapPattern(self, f, PhantomData)
    }
    fn and_then<F, T>(self, f: F) -> AndThenPattern<R, Self, F>
        where Self: Sized,
              F: FnOnce(Self::Output) -> T,
              T: ReadPattern<R>
    {
        AndThenPattern(self, f, PhantomData)
    }
}

pub struct BoxPattern<R, O>(Box<FnMut(R) -> IoFuture<(R, O)> + Send + 'static>);
impl<R: io::Read, O> ReadPattern<R> for BoxPattern<R, O> {
    type Output = O;
    type Future = IoFuture<(R, O)>;
    fn read_pattern(mut self, reader: R) -> Self::Future {
        (self.0)(reader)
    }
}

pub struct MapPattern<R, P, F>(P, F, PhantomData<R>);
impl<R: io::Read, P, F, U> ReadPattern<R> for MapPattern<R, P, F>
    where P: ReadPattern<R>,
          P::Future: Send + 'static,
          F: FnOnce(P::Output) -> U + Send + 'static
{
    type Output = U;
    type Future = IoFuture<(R, Self::Output)>;
    fn read_pattern(self, reader: R) -> Self::Future {
        let MapPattern(p, f, _) = self;
        p.read_pattern(reader).map(move |(r, v)| (r, f(v))).boxed()
    }
}

pub struct AndThenPattern<R, P, F>(P, F, PhantomData<R>);
impl<R: io::Read, P, F, U> ReadPattern<R> for AndThenPattern<R, P, F>
    where P: ReadPattern<R>,
          P::Future: Send + 'static,
          F: FnOnce(P::Output) -> U + Send + 'static,
          U: ReadPattern<R>,
          U::Future: Send + 'static
{
    type Output = U::Output;
    type Future = IoFuture<(R, Self::Output)>;
    fn read_pattern(self, reader: R) -> Self::Future {
        let AndThenPattern(p, f, _) = self;
        p.read_pattern(reader).and_then(move |(r, v)| f(v).read_pattern(r)).boxed()
    }
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

impl<R: io::Read, T> ReadPattern<R> for pattern::read::Str<T>
    where R: Send + 'static,
          T: ReadPattern<R>,
          T::Future: Send + 'static,
          T::Output: pattern::read::AsUsize
{
    type Output = String;
    type Future = IoFuture<(R, Self::Output)>;
    fn read_pattern(self, reader: R) -> Self::Future {
        use pattern::read::AsUsize;
        self.0
            .read_pattern(reader)
            .and_then(|(reader, len)| reader.async_read_exact(vec![0; len.as_usize()]))
            .and_then(|(reader, buf)| {
                String::from_utf8(buf)
                    .map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("Invalid UTF-8: {}", e))
                    })
                    .map(|s| (reader, s))
            })
            .boxed()
    }
}

impl<R: io::Read, N, T> ReadPattern<R> for pattern::read::Vector<N, T>
    where R: Send + 'static,
          N: ReadPattern<R>,
          N::Future: Send + 'static,
          N::Output: pattern::read::AsUsize,
          T: ReadPattern<R> + Clone + Send + 'static,
          T::Future: Send + 'static,
          T::Output: Send + 'static
{
    type Output = Vec<T::Output>;
    type Future = IoFuture<(R, Self::Output)>;
    fn read_pattern(self, reader: R) -> Self::Future {
        use pattern::read::AsUsize;
        let pattern::read::Vector(len, elem) = self;
        len.read_pattern(reader)
            .and_then(move |(reader, len)| ReadVec::new(reader, len.as_usize(), elem))
            .boxed()
    }
}

struct ReadVec<R, T, F, O>(Result<F, Option<R>>, T, Vec<O>);
impl<R, T> ReadVec<R, T, T::Future, T::Output>
    where R: io::Read,
          T: ReadPattern<R> + Clone
{
    fn new(reader: R, size: usize, pattern: T) -> Self {
        if size == 0 {
            ReadVec(Err(Some(reader)), pattern, Vec::new())
        } else {
            let v = Vec::with_capacity(size);
            let f = pattern.clone().read_pattern(reader);
            ReadVec(Ok(f), pattern, v)
        }
    }
}
impl<R, T> Future for ReadVec<R, T, T::Future, T::Output>
    where R: io::Read,
          T: ReadPattern<R> + Clone
{
    type Item = (R, Vec<T::Output>);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use std::mem;
        match mem::replace(&mut self.0, Err(None)) {
            Ok(mut future) => {
                if let Async::Ready((r, e)) = future.poll()? {
                    self.2.push(e);
                    if self.2.len() == self.2.capacity() {
                        self.0 = Err(Some(r));
                    } else {
                        let f = self.1.clone().read_pattern(r);
                        self.0 = Ok(f);
                    }
                    self.poll()
                } else {
                    self.0 = Ok(future);
                    Ok(Async::NotReady)
                }
            }
            Err(reader) => {
                if let Some(reader) = reader {
                    Ok(Async::Ready((reader, mem::replace(&mut self.2, Vec::new()))))
                } else {
                    panic!("Cannot poll ReadVec twice");
                }
            }
        }
    }
}

impl<R: io::Read> ReadPattern<R> for () {
    type Output = ();
    type Future = futures::Finished<(R, Self::Output), io::Error>;
    fn read_pattern(self, reader: R) -> Self::Future {
        futures::finished((reader, ()))
    }
}

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

impl<R: io::Read> ReadPattern<R> for io::Error {
    type Output = ();
    type Future = futures::Failed<(R, Self::Output), io::Error>;
    fn read_pattern(self, _reader: R) -> Self::Future {
        futures::failed(self)
    }
}

impl<R: io::Read, T> ReadPattern<R> for pattern::read::Immediate<T> {
    type Output = T;
    type Future = futures::Finished<(R, Self::Output), io::Error>;
    fn read_pattern(self, reader: R) -> Self::Future {
        futures::finished((reader, self.0))
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
