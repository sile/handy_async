use std::io;
use futures::Future;
use futures::BoxFuture;
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

pub trait FixedLenPattern<R>: Pattern<R>
    where R: AsyncRead
{
    type Buffer: AsMut<[u8]>;
    fn from_buf(buf: Self::Buffer) -> Self::Output;
}

pub struct U8;
impl<R> Pattern<R> for U8
    where R: AsyncRead
{
    type Output = u8;
    type Future = ReadFixedLenPattern<R, Self>;
    fn read_pattern(self, reader: R::State) -> Self::Future {
        ReadFixedLenPattern::new(reader, [0; 1])
    }
}
impl<R> FixedLenPattern<R> for U8
    where R: AsyncRead
{
    type Buffer = [u8; 1];
    fn from_buf(buf: Self::Buffer) -> Self::Output {
        buf[0]
    }
}

pub struct ReadFixedLenPattern<R, P>
    where R: AsyncRead,
          P: FixedLenPattern<R>
{
    future: ReadExact<R, P::Buffer>,
}
impl<R, P> ReadFixedLenPattern<R, P>
    where R: AsyncRead,
          P: FixedLenPattern<R>
{
    fn new(reader: R::State, buf: P::Buffer) -> Self {
        ReadFixedLenPattern { future: read_exact(reader, buf) }
    }
}
impl<R, P> Future for ReadFixedLenPattern<R, P>
    where R: AsyncRead,
          P: FixedLenPattern<R>
{
    type Item = (R::State, P::Output);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (reader, buf) = try_ready!(self.future.poll());
        Ok(Async::Ready((reader, P::from_buf(buf))))
    }
}

impl<R, P1, P2> Pattern<R> for (P1, P2)
    where R: AsyncRead,
          P1: Pattern<R>,
          P2: Pattern<R> + Send + 'static,
          P1::Future: Send + 'static,
          P2::Future: Send + 'static,
          P1::Output: Send + 'static
{
    type Output = (P1::Output, P2::Output);
    type Future = BoxFuture<(R::State, Self::Output), io::Error>;
    fn read_pattern(self, reader: R::State) -> Self::Future {
        let (p1, p2) = self;
        p1.read_pattern(reader)
            .and_then(move |(reader, o1)| {
                p2.read_pattern(reader).map(|(reader, o2)| (reader, (o1, o2)))
            })
            .boxed()
    }
}
