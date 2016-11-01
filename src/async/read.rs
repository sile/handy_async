use std::io;
use futures;
use futures::Poll;
use futures::Async;
use futures::Future;

use buffer::BufferLike;
use pattern::Pattern;

pub trait Readable: Sized + io::Read {
    type Result: Future<Item = Self, Error = io::Error>;
    fn readable(self) -> Self::Result;
}
impl<'a> Readable for &'a [u8] {
    type Result = futures::Finished<Self, io::Error>;
    fn readable(self) -> Self::Result {
        futures::finished(self)
    }
}
impl<T> Readable for io::Cursor<T>
    where T: AsRef<[u8]>
{
    type Result = futures::Finished<Self, io::Error>;
    fn readable(self) -> Self::Result {
        futures::finished(self)
    }
}

pub fn read_exact<R, T>(reader: R, buf: T) -> ReadExact<R, T>
    where R: Readable,
          T: AsMut<[u8]>
{
    ReadExact {
        reader: reader.readable(),
        buffer: Some(buf),
        offset: 0,
    }
}

pub struct ReadExact<R, T>
    where R: Readable
{
    reader: R::Result,
    buffer: Option<T>,
    offset: usize,
}
impl<R, T> Future for ReadExact<R, T>
    where R: Readable,
          T: AsMut<[u8]>
{
    type Item = (R, T);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready(mut reader) = try!(self.reader.poll()) {
            let mut buf = self.buffer.take().unwrap();
            let read_size = try!(reader.read(&mut buf.as_mut()[self.offset..]));
            self.offset += read_size;
            if read_size == 0 {
                let description = format!("Expected: {} bytes", buf.as_mut().len());
                Err(io::Error::new(io::ErrorKind::UnexpectedEof, description))
            } else if self.offset == buf.as_mut().len() {
                Ok(Async::Ready((reader, buf)))
            } else {
                self.reader = reader.readable();
                self.buffer = Some(buf);
                Ok(Async::NotReady)
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}

pub fn read_value<R, T>(reader: R, buf: T) -> ReadValue<R, T>
    where R: Readable,
          T: BufferLike
{
    ReadValue { future: read_exact(reader, buf) }
}

pub struct ReadValue<R, T>
    where R: Readable
{
    future: ReadExact<R, T>,
}
impl<R, T> Future for ReadValue<R, T>
    where R: Readable,
          T: BufferLike
{
    type Item = (R, T::Value);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((reader, buf)) = try!(self.future.poll()) {
            Ok(Async::Ready((reader, buf.into_value())))
        } else {
            Ok(Async::NotReady)
        }
    }
}

pub trait ReadPattern: Pattern {
    fn read_pattern<R>(self, reader: R) -> ReadValue<R, Self::Buffer>
        where R: Readable
    {
        read_value(reader, self.into_buffer())
    }
}
impl<T> ReadPattern for T where T: Pattern {}

pub struct StatefulReader<R, T> {
    reader: R,
    pub state: T,
}
impl<R, T> StatefulReader<R, T>
    where R: Readable
{
    pub fn new(reader: R, state: T) -> Self {
        StatefulReader {
            reader: reader,
            state: state,
        }
    }
    pub fn read_exact<B>(self, buf: B) -> ReadExact<Self, B>
        where B: AsMut<[u8]>
    {
        read_exact(self, buf)
    }
    pub fn read_pattern<P>(self, pattern: P) -> ReadValue<Self, P::Buffer>
        where P: Pattern
    {
        pattern.read_pattern(self)
    }
}
impl<R, T> io::Read for StatefulReader<R, T>
    where R: io::Read
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}
impl<R, T> Readable for StatefulReader<R, T>
    where R: Readable
{
    type Result = StatefulReadable<R, T>;
    fn readable(self) -> Self::Result {
        StatefulReadable {
            reader: self.reader.readable(),
            state: Some(self.state),
        }
    }
}

pub struct StatefulReadable<R, T>
    where R: Readable
{
    reader: R::Result,
    state: Option<T>,
}
impl<R, T> Future for StatefulReadable<R, T>
    where R: Readable
{
    type Item = StatefulReader<R, T>;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready(reader) = try!(self.reader.poll()) {
            Ok(Async::Ready(StatefulReader::new(reader, self.state.take().unwrap())))
        } else {
            Ok(Async::NotReady)
        }
    }
}

#[cfg(test)]
mod test {
    use futures::Future;
    use pattern::read::U8;
    use super::*;

    #[test]
    fn it_works() {
        let reader = b"abc";
        assert_eq!(U8.read_pattern(&reader[..]).wait().ok().unwrap().1, b'a');
    }
}
