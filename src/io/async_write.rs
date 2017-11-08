use std::io::{Write, Error, ErrorKind};
use futures::{Poll, Async, Future};

use pattern::Window;
use super::AsyncIoError;

/// An asynchronous version of the standard `Write` trait.
///
/// Since this is assumed as a basic building block,
/// it may be more convenient to use [`WriteInto`](./trait.WriteInto.html) for ordinary cases.
///
/// # Notice
///
/// For executing asynchronously, we assume the writer which implements this trait
/// returns the `std::io::ErrorKind::WouldBlock` error
/// if a write operation would be about to block.
pub trait AsyncWrite: Write + Sized {
    /// Creates a future which will write bytes asynchronously.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate futures;
    /// # extern crate handy_async;
    /// use futures::Future;
    /// use handy_async::io::AsyncWrite;
    ///
    /// # fn main() {
    /// let (output, _, _) = vec![].async_write(b"hello").wait().ok().unwrap();
    /// assert_eq!(&output[..], b"hello");
    ///
    /// let mut output = [0; 3];
    /// let (_, _, _) = (&mut output).async_write(b"hello").wait().ok().unwrap();
    /// assert_eq!(&output[..], b"hel");
    ///
    /// let (_, _, written_size) = [0; 0].async_write(b"hello").wait().ok().unwrap();
    /// assert_eq!(written_size, 0);
    /// # }
    /// ```
    fn async_write<B: AsRef<[u8]>>(self, buf: B) -> WriteBytes<Self, B> {
        WriteBytes(Some((self, buf)))
    }

    /// Creates a future which will write all bytes asynchronously.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate futures;
    /// # extern crate handy_async;
    /// use std::io::ErrorKind;
    /// use futures::Future;
    /// use handy_async::io::AsyncWrite;
    ///
    /// # fn main() {
    /// let (output, _) = vec![].async_write_all(b"hello").wait().ok().unwrap();
    /// assert_eq!(&output[..], b"hello");
    ///
    /// let mut output = [0; 3];
    /// let e = (&mut output).async_write_all(b"hello").wait().err().unwrap();
    /// assert_eq!(e.error_ref().kind(), ErrorKind::UnexpectedEof);
    /// # }
    fn async_write_all<B: AsRef<[u8]>>(self, buf: B) -> WriteAll<Self, B> {
        WriteAll(self.async_write(Window::new_ref(buf)))
    }

    /// Creates a future which will flush the internal buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate futures;
    /// # extern crate handy_async;
    /// use std::io::BufWriter;
    /// use futures::Future;
    /// use handy_async::io::AsyncWrite;
    ///
    /// # fn main() {
    /// let writer = BufWriter::new(vec![]);
    /// let (writer, _) = writer.async_write_all(b"hello").wait().ok().unwrap();
    /// assert_eq!(writer.get_ref(), b"");
    ///
    /// let writer = writer.async_flush().wait().ok().unwrap();
    /// assert_eq!(writer.get_ref(), b"hello");
    /// # }
    /// ```
    fn async_flush(self) -> Flush<Self> {
        Flush(Some(self))
    }
}
impl<W: Write> AsyncWrite for W {}

/// A future which will write bytes to `W`.
///
/// This is created by calling `AsyncWrite::async_write` method.
#[derive(Debug)]
pub struct WriteBytes<W, B>(Option<(W, B)>);
impl<W: Write, B: AsRef<[u8]>> Future for WriteBytes<W, B> {
    type Item = (W, B, usize);
    type Error = AsyncIoError<(W, B)>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (mut w, b) = self.0.take().expect("Cannot poll WriteBytes twice");
        match w.write(b.as_ref()) {
            Ok(write_size) => Ok(Async::Ready((w, b, write_size))),
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    self.0 = Some((w, b));
                    Ok(Async::NotReady)
                } else {
                    Err(AsyncIoError::new((w, b), e))
                }
            }
        }
    }
}

/// A future which will write all bytes to `W`.
///
/// This is created by calling `AsyncWrite::async_write_all` method.
#[derive(Debug)]
pub struct WriteAll<W, B>(WriteBytes<W, Window<B>>);
impl<W: Write, B: AsRef<[u8]>> Future for WriteAll<W, B> {
    type Item = (W, B);
    type Error = AsyncIoError<(W, B)>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready((w, b, size)) =
            self.0.poll().map_err(
                |e| e.map_state(|(w, b)| (w, b.into_inner())),
            )?
        {
            let b = b.skip(size);
            if b.as_ref().is_empty() {
                return Ok(Async::Ready((w, b.into_inner())));
            } else if size == 0 {
                let e = Error::new(
                    ErrorKind::UnexpectedEof,
                    format!("Unexpected EOF (remaining {} bytes", b.as_ref().len()),
                );
                return Err(AsyncIoError::new((w, b.into_inner()), e));
            } else {
                self.0 = w.async_write(b);
            }
        }
        Ok(Async::NotReady)
    }
}

/// A future which will flush the internal buffer of `W`.
///
/// This is created by calling `AsyncWrite::async_flush` method.
#[derive(Debug)]
pub struct Flush<W>(Option<W>);
impl<W: Write> Future for Flush<W> {
    type Item = W;
    type Error = AsyncIoError<W>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut w = self.0.take().expect("Cannot poll Flush twice");
        match w.flush() {
            Ok(()) => Ok(Async::Ready(w)),
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    self.0 = Some(w);
                    Ok(Async::NotReady)
                } else {
                    Err(AsyncIoError::new(w, e))
                }
            }
        }
    }
}
