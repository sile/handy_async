use std::io::{Read, Error, ErrorKind};
use futures::{Poll, Async, Future};

use pattern::Window;
use super::AsyncIoError;

/// An asynchronous version of the standard `Read` trait.
///
/// Since this is assumed as a basic building block,
/// it may be more convenient to use `ReadPattern` for ordinary cases.
///
/// # Notice
///
/// For executing asynchronously, we assume the reader which implements this trait returns the
/// `std::io::ErrorKind::WouldBlock` error if a read operation would be about to block.
pub trait AsyncRead: Read + Sized {
    /// Creates a future which will read bytes asynchronously.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate futures;
    /// # extern crate handy_async;
    /// use std::io::empty;
    /// use futures::Future;
    /// use handy_async::io::AsyncRead;
    ///
    /// # fn main() {
    /// let (_, buf, read_size) = b"hello".async_read([0; 8]).wait().ok().unwrap();
    /// assert_eq!(read_size, 5);
    /// assert_eq!(&buf[0..5], b"hello");
    ///
    /// let (_, _, read_size) = empty().async_read([0; 4]).wait().ok().unwrap();
    /// assert_eq!(read_size, 0);
    /// # }
    /// ```
    fn async_read<B: AsMut<[u8]>>(self, buf: B) -> ReadBytes<Self, B> {
        ReadBytes(Some((self, buf)))
    }

    /// Creates a future which will read non empty bytes asynchronously.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate futures;
    /// # extern crate handy_async;
    /// use std::io::{empty, ErrorKind};
    /// use futures::Future;
    /// use handy_async::io::AsyncRead;
    ///
    /// # fn main() {
    /// let (_, buf, read_size) = b"hello".async_read_non_empty([0; 8]).wait().ok().unwrap();
    /// assert_eq!(read_size, 5);
    /// assert_eq!(&buf[0..5], b"hello");
    ///
    /// let e = empty().async_read_non_empty([0; 4]).wait().err().unwrap();
    /// assert_eq!(e.error_ref().kind(), ErrorKind::UnexpectedEof);
    /// # }
    /// ```
    fn async_read_non_empty<B: AsMut<[u8]>>(self, buf: B) -> ReadNonEmpty<Self, B> {
        ReadNonEmpty(self.async_read(buf))
    }

    /// Creates a future which will read exact bytes asynchronously.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate futures;
    /// # extern crate handy_async;
    /// use std::io::ErrorKind;
    /// use futures::Future;
    /// use handy_async::io::AsyncRead;
    ///
    /// # fn main() {
    /// let (_, buf) = b"hello".async_read_exact([0; 3]).wait().ok().unwrap();
    /// assert_eq!(&buf[..], b"hel");
    ///
    /// let e = b"hello".async_read_exact([0; 8]).wait().err().unwrap();
    /// assert_eq!(e.error_ref().kind(), ErrorKind::UnexpectedEof);
    /// # }
    /// ```
    fn async_read_exact<B: AsMut<[u8]>>(self, buf: B) -> ReadExact<Self, B> {
        ReadExact(self.async_read_non_empty(Window::new_mut(buf)))
    }
}
impl<R: Read> AsyncRead for R {}

/// A future which will read bytes from `R`.
///
/// This is created by calling `AsyncRead::async_read` method.
#[derive(Debug)]
pub struct ReadBytes<R, B>(Option<(R, B)>);
impl<R, B> ReadBytes<R, B> {
    /// Returns the reference to the reader.
    pub fn reader(&self) -> &R {
        &self.0.as_ref().expect("ReadBytes has been consumed").0
    }

    /// Returns the mutable reference to the reader.
    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.0.as_mut().expect("ReadBytes has been consumed").0
    }
}
impl<R: Read, B: AsMut<[u8]>> Future for ReadBytes<R, B> {
    type Item = (R, B, usize);
    type Error = AsyncIoError<(R, B)>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (mut inner, mut buf) = self.0.take().expect("Cannot poll ReadBytes twice");
        match inner.read(buf.as_mut()) {
            Ok(size) => Ok(Async::Ready((inner, buf, size))),
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    self.0 = Some((inner, buf));
                    Ok(Async::NotReady)
                } else {
                    Err(AsyncIoError::new((inner, buf), e))
                }
            }
        }
    }
}

/// A future which will read non empty bytes from `R`.
///
/// This is created by calling `AsyncRead::async_read_non_empty` method.
#[derive(Debug)]
pub struct ReadNonEmpty<R, B>(ReadBytes<R, B>);
impl<R, B> ReadNonEmpty<R, B> {
    /// Returns the reference to the reader.
    pub fn reader(&self) -> &R {
        self.0.reader()
    }

    /// Returns the mutable reference to the reader.
    pub fn reader_mut(&mut self) -> &mut R {
        self.0.reader_mut()
    }
}
impl<R: Read, B: AsMut<[u8]>> Future for ReadNonEmpty<R, B> {
    type Item = (R, B, usize);
    type Error = AsyncIoError<(R, B)>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((r, mut b, size)) = self.0.poll()? {
            if size == 0 && !b.as_mut().is_empty() {
                let e = Error::new(
                    ErrorKind::UnexpectedEof,
                    format!("Unexpected Eof ({} bytes are required)", b.as_mut().len()),
                );
                Err(AsyncIoError::new((r, b), e))
            } else {
                Ok(Async::Ready((r, b, size)))
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}

/// A future which will read bytes required to fill the buffer `B` completely.
///
/// This is created by calling `AsyncRead::async_read_exact` method.
#[derive(Debug)]
pub struct ReadExact<R, B>(ReadNonEmpty<R, Window<B>>);
impl<R, B> ReadExact<R, B> {
    /// Returns the reference to the reader.
    pub fn reader(&self) -> &R {
        self.0.reader()
    }

    /// Returns the mutable reference to the reader.
    pub fn reader_mut(&mut self) -> &mut R {
        self.0.reader_mut()
    }
}
impl<R, B> Future for ReadExact<R, B>
where
    R: Read,
    B: AsMut<[u8]>,
{
    type Item = (R, B);
    type Error = AsyncIoError<(R, B)>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready((r, b, read_size)) =
            self.0.poll().map_err(
                |e| e.map_state(|(r, b)| (r, b.into_inner())),
            )?
        {
            let mut b = b.skip(read_size);
            if b.as_mut().is_empty() {
                return Ok(Async::Ready((r, b.into_inner())));
            } else {
                self.0 = r.async_read_non_empty(b);
            }
        }
        Ok(Async::NotReady)
    }
}
