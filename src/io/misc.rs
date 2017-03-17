//! Miscellaneous I/O components.
use std::io::{self, Read, Write, Result};

/// `Counter` counts the number of read/write bytes issued to an underlying stream.
///
/// # Examples
///
/// ```
/// use std::io::{self, Write};
/// use handy_async::io::misc::Counter;
///
/// let mut writer = Counter::new(io::sink());
/// writer.write(b"Hello").unwrap();
/// writer.write(b"World!").unwrap();
/// assert_eq!(writer.written_size(), b"Hello".len() + b"World!".len());
/// ```
pub struct Counter<T> {
    inner: T,
    read_size: usize,
    written_size: usize,
}
impl<T> Counter<T> {
    /// Makes a new `Counter` which has the inner stream `T`.
    pub fn new(inner: T) -> Self {
        Counter {
            inner: inner,
            read_size: 0,
            written_size: 0,
        }
    }

    /// Gets a reference to the underlying stream.
    pub fn inner_ref(&self) -> &T {
        &self.inner
    }

    /// Gets a mutable reference to the underlying stream.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Unwraps this `Counter`, returning the underlying stream.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Returns the total byte size written to the underlying stream.
    pub fn written_size(&self) -> usize {
        self.written_size
    }

    /// Returns the total byte size read from the underlying stream.
    pub fn read_size(&self) -> usize {
        self.read_size
    }
}
impl Counter<io::Sink> {
    /// Equivalent to `Counter::new(std::io::sink())`.
    pub fn with_sink() -> Self {
        Self::new(io::sink())
    }
}
impl<T: Read> Read for Counter<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.read(buf).map(|size| {
                                     self.read_size += size;
                                     size
                                 })
    }
}
impl<T: Write> Write for Counter<T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write(buf).map(|size| {
                                      self.written_size += size;
                                      size
                                  })
    }
    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}
