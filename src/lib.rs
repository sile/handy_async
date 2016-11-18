extern crate futures;
extern crate byteorder;

use std::io;

pub mod sync;
pub mod async;
pub mod pattern;

#[derive(Debug)]
pub struct ReadableState<T, R> {
    state: T,
    reader: R,
}
impl<T, R> ReadableState<T, R>
    where R: io::Read
{
    pub fn new(state: T, reader: R) -> Self {
        ReadableState {
            state: state,
            reader: reader,
        }
    }
}
impl<T, R> ReadableState<T, R> {
    pub fn unwrap(self) -> (T, R) {
        (self.state, self.reader)
    }
    pub fn state_ref(&self) -> &T {
        &self.state
    }
    pub fn state_mut(&mut self) -> &mut T {
        &mut self.state
    }
    pub fn reader_ref(&self) -> &R {
        &self.reader
    }
    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }
}
impl<T, R> io::Read for ReadableState<T, R>
    where R: io::Read
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

#[derive(Debug)]
pub struct WritableState<T, W> {
    state: T,
    writer: W,
}
impl<T, W> WritableState<T, W>
    where W: io::Write
{
    pub fn new(state: T, writer: W) -> Self {
        WritableState {
            state: state,
            writer: writer,
        }
    }
}
impl<T, W> WritableState<T, W> {
    pub fn unwrap(self) -> (T, W) {
        (self.state, self.writer)
    }
    pub fn state_ref(&self) -> &T {
        &self.state
    }
    pub fn state_mut(&mut self) -> &mut T {
        &mut self.state
    }
    pub fn writer_ref(&self) -> &W {
        &self.writer
    }
    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }
}
impl<T, W> io::Write for WritableState<T, W>
    where W: io::Write
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
