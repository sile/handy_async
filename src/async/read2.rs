use std::io;
use futures::Future;
use futures::Poll;
use futures::Async;

macro_rules! try_ready {
    ($e:expr) => {
        if let Async::Ready(v) = try!($e) {
            v
        } else {
            return Ok(Async::NotReady);
        }
    }
}

pub trait AsyncRead: io::Read {
    type State: AsMut<Self>;
    type Future: Future<Item = Self::State, Error = io::Error>;
    fn on_read_ready(state: Self::State) -> Self::Future;
}

pub trait ReadableState<R>: Sized + AsMut<R>
    where R: AsyncRead<State = Self>
{
    fn read<B>(self, buf: B) -> Read<R, B>
        where B: AsMut<[u8]>
    {
        read(self, buf)
    }
    fn read_exact<B>(self, buf: B) -> ReadExact<R, B>
        where B: AsMut<[u8]>
    {
        read_exact(self, buf)
    }
    fn read_pattern<P>(self, pattern: P) -> P::Future
        where P: Pattern<R>
    {
        pattern.read_pattern(self)
    }
}

pub fn read<R, B>(reader: R::State, buf: B) -> Read<R, B>
    where R: AsyncRead,
          B: AsMut<[u8]>
{
    Read {
        future: R::on_read_ready(reader),
        buffer: Some(buf),
    }
}
pub struct Read<R, B>
    where R: AsyncRead
{
    future: R::Future,
    buffer: Option<B>,
}
impl<R, B> Future for Read<R, B>
    where R: AsyncRead,
          B: AsMut<[u8]>
{
    type Item = (R::State, B, usize);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut reader = try_ready!(self.future.poll());
        let mut buf = self.buffer.take().unwrap();
        let read_size = try!(reader.as_mut().read(buf.as_mut()));
        Ok(Async::Ready((reader, buf, read_size)))
    }
}

struct Window<B> {
    inner: B,
    offset: usize,
}
impl<B> AsMut<[u8]> for Window<B>
    where B: AsMut<[u8]>
{
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[self.offset..]
    }
}

pub fn read_exact<R, B>(reader: R::State, buf: B) -> ReadExact<R, B>
    where R: AsyncRead,
          B: AsMut<[u8]>
{
    ReadExact {
        future: read(reader,
                     Window {
                         inner: buf,
                         offset: 0,
                     }),
    }
}
pub struct ReadExact<R, B>
    where R: AsyncRead
{
    future: Read<R, Window<B>>,
}
impl<R, B> Future for ReadExact<R, B>
    where R: AsyncRead,
          B: AsMut<[u8]>
{
    type Item = (R::State, B);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (reader, mut buf, read_size) = try_ready!(self.future.poll());
        buf.offset += read_size;
        if read_size == 0 {
            Err(io::Error::new(io::ErrorKind::UnexpectedEof, "early eof"))
        } else if buf.as_mut().is_empty() {
            Ok(Async::Ready((reader, buf.inner)))
        } else {
            self.future = read(reader, buf);
            Ok(Async::NotReady)
        }
    }
}

pub trait Pattern<R>
    where R: AsyncRead
{
    type Output;
    type Future: Future<Item = (R::State, Self::Output), Error = io::Error>;
    fn read_pattern(self, reader: R::State) -> Self::Future;
}

pub struct U8;
impl<R> Pattern<R> for U8
    where R: AsyncRead
{
    type Output = u8;
    type Future = ReadU8<R>;
    fn read_pattern(self, reader: R::State) -> Self::Future {
        let buf = [0; 1];
        ReadU8(read_exact(reader, buf))
    }
}

pub struct ReadU8<R>(ReadExact<R, [u8; 1]>) where R: AsyncRead;
impl<R> Future for ReadU8<R>
    where R: AsyncRead
{
    type Item = (R::State, u8);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (reader, buf) = try_ready!(self.0.poll());
        Ok(Async::Ready((reader, buf[0])))
    }
}

impl<R, P1, P2> Pattern<R> for (P1, P2)
    where R: AsyncRead,
          P1: Pattern<R>,
          P2: Pattern<R>
{
    type Output = (P1::Output, P2::Output);
    type Future = ReadTuple2<R, P1, P2>;
    fn read_pattern(self, reader: R::State) -> Self::Future {
        ReadTuple2 {
            ps: ((), Some(self.1)),
            fs: (self.0.read_pattern(reader), None),
            os: (None, ()),
        }
    }
}

pub struct ReadTuple2<R, P1, P2>
    where R: AsyncRead,
          P1: Pattern<R>,
          P2: Pattern<R>
{
    ps: ((), Option<P2>),
    fs: (P1::Future, Option<P2::Future>),
    os: (Option<P1::Output>, ()),
}
impl<R, P1, P2> Future for ReadTuple2<R, P1, P2>
    where R: AsyncRead,
          P1: Pattern<R>,
          P2: Pattern<R>
{
    type Item = (R::State, (P1::Output, P2::Output));
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.fs {
            (_, Some(ref mut f)) => {
                let (reader, o) = try_ready!(f.poll());
                Ok(Async::Ready((reader, (self.os.0.take().unwrap(), o))))
            }
            (ref mut f, _) => {
                let (reader, o) = try_ready!(f.poll());
                self.os.0 = Some(o);
                self.fs.1 = Some(self.ps.1.take().unwrap().read_pattern(reader));
                Ok(Async::NotReady)
            }
        }
    }
}
