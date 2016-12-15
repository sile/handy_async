use std;
use std::io::{Read, Error, ErrorKind, Result};
use futures::{Poll, Async, Future, Stream};
use byteorder::{ByteOrder, NativeEndian, BigEndian, LittleEndian};

use io::AsyncRead;
use io::futures::{ReadBytes, ReadExact, ReadNonEmpty};
use pattern::{Pattern, Buf, Window};
use pattern::read;
use pattern::combinators::{self, BE, LE, PartialBuf};
use matcher::{AsyncMatch, Matcher};
use matcher::streams::MatchStream;
use super::AsyncIoError;

/// A matcher to read patterns from the inner reader `R`.
///
/// This is mainly used to define your own reading patterns.
/// See the example of the [ReadFrom](./trait.ReadFrom.html) trait.
pub struct PatternReader<R>(R);
impl<R: Read> PatternReader<R> {
    /// Makes new `PatternReader` instance.
    pub fn new(inner: R) -> Self {
        PatternReader(inner)
    }
}
impl<R: Read> Read for PatternReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.0.read(buf)
    }
}
impl<R> Matcher for PatternReader<R> {
    type Error = Error;
}

/// The `ReadFrom` trait allows for reading a value of the pattern from a source asynchronously.
///
/// # Notice
///
/// For executing asynchronously, we assume the writer `R` returns
/// `the std::io::ErrorKind::WouldBlock` error if a read operation would be about to block.
///
/// # Examples
///
/// Defines your own reading pattern:
///
/// ```
/// # extern crate futures;
/// # extern crate handy_async;
/// use std::io::{Read, Error, ErrorKind};
/// use futures::{Future, BoxFuture};
/// use handy_async::io::{ReadFrom, PatternReader, AsyncIoError};
/// use handy_async::pattern::Pattern;
/// use handy_async::matcher::AsyncMatch;
///
/// // Defines pattern.
/// struct HelloWorld;
/// impl Pattern for HelloWorld {
///    type Value = Vec<u8>;
/// }
///
/// // Implements pattern maching between `PatternReader<R>` and `HelloWorld`.
/// impl<R: Read + Send + 'static> AsyncMatch<PatternReader<R>> for HelloWorld {
///     type Future = BoxFuture<(PatternReader<R>, Vec<u8>), AsyncIoError<PatternReader<R>>>;
///     fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
///         let buf = vec![0; b"Hello World!".len()];
///         buf.and_then(|b| if b == b"Hello World!" {
///             Ok(b)
///         } else {
///             Err(Error::new(ErrorKind::InvalidData, format!("Unexpected bytes {:?}", b)) )
///         }).async_match(matcher).boxed()
///     }
/// }
///
/// # fn main() {
/// // Executes pattern matchings.
///
/// // matched
/// let pattern = (vec![0; 5], HelloWorld);
/// let (rest, value) = pattern.read_from(&b"Hey! Hello World!!!"[..]).wait().unwrap();
/// assert_eq!(value.0, b"Hey! ");
/// assert_eq!(value.1, b"Hello World!");
/// assert_eq!(rest, b"!!");
///
/// // unmatched
/// let pattern = (vec![0; 5], HelloWorld);
/// let e = pattern.read_from(&b"Hey! Hello Rust!!!"[..]).wait().err().unwrap();
/// assert_eq!(e.error_ref().kind(), ErrorKind::InvalidData);
/// # }
/// ```
pub trait ReadFrom<R: Read>: AsyncMatch<PatternReader<R>> {
    /// Creates a future instance to read a value of the pattern from `reader`.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate futures;
    /// # extern crate handy_async;
    /// use handy_async::io::ReadFrom;
    /// use handy_async::pattern::{Pattern, Endian};
    /// use handy_async::pattern::read::{U8, U16};
    /// use futures::Future;
    ///
    /// # fn main() {
    /// let mut input = &[1, 0, 2][..];
    /// let pattern = (U8, U16.be());
    /// let future = pattern.read_from(&mut input);
    /// assert_eq!(future.wait().unwrap().1, (1, 2));
    /// # }
    /// ```
    fn read_from(self, reader: R) -> ReadPattern<Self, R> {
        ReadPattern(self.async_match(PatternReader(reader)))
    }

    /// Consumes this pattern and the `reader`,
    /// returning a stream which will produce a sequence of read values.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate futures;
    /// # extern crate handy_async;
    /// use futures::{Future, Stream};
    /// use handy_async::pattern::read::U8;
    /// use handy_async::io::ReadFrom;
    ///
    /// # fn main() {
    /// let values = U8.into_stream(&b"hello"[..]).take(3).collect().wait().unwrap();
    /// assert_eq!(values, b"hel");
    /// # }
    /// ```
    fn into_stream(self, reader: R) -> ReadStream<R, Self>
        where Self: Clone
    {
        ReadStream(AsyncMatch::into_stream(self, PatternReader(reader)))
    }
}
impl<R: Read, T> ReadFrom<R> for T where T: AsyncMatch<PatternReader<R>> {}

/// Stream to produce a sequence of read values.
///
/// This is created by calling `ReadFrom::into_stream` method.
pub struct ReadStream<R: Read, P>(MatchStream<PatternReader<R>, P>)
    where P: AsyncMatch<PatternReader<R>>;
impl<R: Read, P> Stream for ReadStream<R, P>
    where P: AsyncMatch<PatternReader<R>> + Clone
{
    type Item = P::Value;
    type Error = AsyncIoError<R>;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.0.poll().map_err(|e| e.map_state(|r| r.0))
    }
}

/// Future to match between a pattern `P` and bytes read from `R`.
///
/// This is created by calling `ReadFrom::read_from` method.
pub struct ReadPattern<P, R>(P::Future) where P: AsyncMatch<PatternReader<R>>;
impl<P, R> Future for ReadPattern<P, R>
    where P: AsyncMatch<PatternReader<R>>
{
    type Item = (R, P::Value);
    type Error = AsyncIoError<R>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(self.0.poll().map_err(|e| e.map_state(|m| m.0))?.map(|(m, v)| (m.0, v)))
    }
}

/// A future which will read bytes from `R` to fill the buffer `B` completely.
///
/// This future is generally created by invoking
/// `ReadFrom::read_from` method for buffer like patterns
/// such as the following.
///
/// ```
/// use handy_async::io::ReadFrom;
/// use handy_async::pattern::{Buf, Window};
///
/// vec![0; 32].read_from(std::io::empty());
/// Buf([0; 32]).read_from(std::io::empty());
/// Window::new([0; 32]).skip(4).read_from(std::io::empty());
/// ```
pub struct ReadBuf<R, B>(ReadExact<PatternReader<R>, Buf<B>>);
impl<R: Read, B: AsMut<[u8]>> Future for ReadBuf<R, B> {
    type Item = (PatternReader<R>, B);
    type Error = AsyncIoError<PatternReader<R>>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(self.0.poll().map_err(|e| e.map_state(|(r, _)| r))?.map(|(r, v)| (r, v.0)))
    }
}
impl<R: Read, B: AsMut<[u8]>> AsyncMatch<PatternReader<R>> for Buf<B> {
    type Future = ReadBuf<R, B>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        ReadBuf(matcher.async_read_exact(self))
    }
}
impl<R: Read> AsyncMatch<PatternReader<R>> for Vec<u8> {
    type Future = ReadBuf<R, Self>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        Buf(self).async_match(matcher)
    }
}
impl<R: Read, B: AsMut<[u8]>> AsyncMatch<PatternReader<R>> for Window<B> {
    type Future = ReadBuf<R, Self>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        Buf(self).async_match(matcher)
    }
}

/// A future which will read bytes from `R` to fill the buffer `B`
/// to the extent possible.
///
/// This future is generally created by invoking
/// `ReadFrom::read_from` method for `PartialBuf` pattern
/// such as the following.
///
/// ```
/// # extern crate futures;
/// # extern crate handy_async;
/// use handy_async::io::ReadFrom;
/// use handy_async::pattern::AllowPartial;
/// use futures::Future;
///
/// # fn main() {
/// // `PartialBuf` pattern is created via `allow_partial` method.
/// let pattern = vec![0; 32].allow_partial();
/// let (_, (_, read_size)) = pattern.read_from(&mut &[0; 4][..]).wait().unwrap();
/// assert_eq!(read_size, 4);
/// # }
/// ```
pub struct ReadPartialBuf<R, B>(ReadNonEmpty<PatternReader<R>, B>);
impl<R: Read, B: AsMut<[u8]>> Future for ReadPartialBuf<R, B> {
    type Item = (PatternReader<R>, (B, usize));
    type Error = AsyncIoError<PatternReader<R>>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll().map(|x| x.map(|(r, b, s)| (r, (b, s)))).map_err(|e| e.map_state(|(r, _)| r))
    }
}
impl<R: Read, B: AsMut<[u8]>> AsyncMatch<PatternReader<R>> for PartialBuf<B> {
    type Future = ReadPartialBuf<R, B>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        ReadPartialBuf(matcher.async_read_non_empty(self.0))
    }
}

/// A future which will read `String` from `R`.
///
/// This future is generally created by invoking
/// `ReadFrom::read_from` method for `String` such as the following.
///
/// If the read bytes are not a valid UTF-8 string, the future will return an error.
///
/// ```
/// use handy_async::io::ReadFrom;
///
/// let str_buf = String::from_utf8(vec![0; 32]).unwrap();
/// str_buf.read_from(std::io::empty()); // This returns a `ReadString` instance
/// ```
pub struct ReadString<R>(ReadExact<PatternReader<R>, Vec<u8>>);
impl<R: Read> Future for ReadString<R> {
    type Item = (PatternReader<R>, String);
    type Error = AsyncIoError<PatternReader<R>>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((r, b)) = self.0.poll().map_err(|e| e.map_state(|(r, _)| r))? {
            match String::from_utf8(b) {
                Ok(s) => Ok(Async::Ready((r, s))),
                Err(e) => {
                    Err(AsyncIoError::new(r, Error::new(ErrorKind::InvalidData, Box::new(e))))
                }
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}
impl<R: Read> AsyncMatch<PatternReader<R>> for String {
    type Future = ReadString<R>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        ReadString(matcher.async_read_exact(self.into_bytes()))
    }
}

/// A future which will read a fixnum associated with `P` from `R`.
pub type ReadFixnum<R, P, T> where P: Pattern =
    <combinators::Map<P, fn(P::Value) -> T> as AsyncMatch<PatternReader<R>>>::Future;
macro_rules! impl_read_fixnum_pattern {
    ($pat:ty, $val:ident, $size:expr, $conv:expr) => {
        impl<R: Read> AsyncMatch<PatternReader<R>> for $pat {
            type Future = ReadFixnum<R, Buf<[u8; $size]>, Self::Value>;
            fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
                fn conv(b: [u8; $size]) -> $val {
                    $conv(&b[..]) as $val
                }
                Buf([0; $size]).map(conv as _).async_match(matcher)
            }
        }
    }
}

impl_read_fixnum_pattern!(read::U8, u8, 1, |b: &[u8]| b[0]);
impl_read_fixnum_pattern!(read::I8, i8, 1, |b: &[u8]| b[0]);

impl_read_fixnum_pattern!(read::U16, u16, 2, |b: &[u8]| NativeEndian::read_u16(b));
impl_read_fixnum_pattern!(BE<read::U16>, u16, 2, |b: &[u8]| BigEndian::read_u16(b));
impl_read_fixnum_pattern!(LE<read::U16>, u16, 2, |b: &[u8]| LittleEndian::read_u16(b));
impl_read_fixnum_pattern!(read::I16, i16, 2, |b: &[u8]| NativeEndian::read_i16(b));
impl_read_fixnum_pattern!(BE<read::I16>, i16, 2, |b: &[u8]| BigEndian::read_i16(b));
impl_read_fixnum_pattern!(LE<read::I16>, i16, 2, |b: &[u8]| LittleEndian::read_i16(b));

impl_read_fixnum_pattern!(read::U24, u32, 3, |b: &[u8]| NativeEndian::read_uint(b, 3));
impl_read_fixnum_pattern!(BE<read::U24>, u32, 3, |b: &[u8]| BigEndian::read_uint(b, 3));
impl_read_fixnum_pattern!(LE<read::U24>, u32, 3, |b: &[u8]| LittleEndian::read_uint(b, 3));
impl_read_fixnum_pattern!(read::I24, i32, 3, |b: &[u8]| NativeEndian::read_int(b, 3));
impl_read_fixnum_pattern!(BE<read::I24>, i32, 3, |b: &[u8]| BigEndian::read_int(b, 3));
impl_read_fixnum_pattern!(LE<read::I24>, i32, 3, |b: &[u8]| LittleEndian::read_int(b, 3));

impl_read_fixnum_pattern!(read::U32, u32, 4, |b: &[u8]| NativeEndian::read_u32(b));
impl_read_fixnum_pattern!(BE<read::U32>, u32, 4, |b: &[u8]| BigEndian::read_u32(b));
impl_read_fixnum_pattern!(LE<read::U32>, u32, 4, |b: &[u8]| LittleEndian::read_u32(b));
impl_read_fixnum_pattern!(read::I32, i32, 4, |b: &[u8]| NativeEndian::read_i32(b));
impl_read_fixnum_pattern!(BE<read::I32>, i32, 4, |b: &[u8]| BigEndian::read_i32(b));
impl_read_fixnum_pattern!(LE<read::I32>, i32, 4, |b: &[u8]| LittleEndian::read_i32(b));

impl_read_fixnum_pattern!(read::U40, u64, 5, |b: &[u8]| NativeEndian::read_uint(b, 5));
impl_read_fixnum_pattern!(BE<read::U40>, u64, 5, |b: &[u8]| BigEndian::read_uint(b, 5));
impl_read_fixnum_pattern!(LE<read::U40>, u64, 5, |b: &[u8]| LittleEndian::read_uint(b, 5));
impl_read_fixnum_pattern!(read::I40, i64, 5, |b: &[u8]| NativeEndian::read_int(b, 5));
impl_read_fixnum_pattern!(BE<read::I40>, i64, 5, |b: &[u8]| BigEndian::read_int(b, 5));
impl_read_fixnum_pattern!(LE<read::I40>, i64, 5, |b: &[u8]| LittleEndian::read_int(b, 5));

impl_read_fixnum_pattern!(read::U48, u64, 6, |b: &[u8]| NativeEndian::read_uint(b, 6));
impl_read_fixnum_pattern!(BE<read::U48>, u64, 6, |b: &[u8]| BigEndian::read_uint(b, 6));
impl_read_fixnum_pattern!(LE<read::U48>, u64, 6, |b: &[u8]| LittleEndian::read_uint(b, 6));
impl_read_fixnum_pattern!(read::I48, i64, 6, |b: &[u8]| NativeEndian::read_int(b, 6));
impl_read_fixnum_pattern!(BE<read::I48>, i64, 6, |b: &[u8]| BigEndian::read_int(b, 6));
impl_read_fixnum_pattern!(LE<read::I48>, i64, 6, |b: &[u8]| LittleEndian::read_int(b, 6));

impl_read_fixnum_pattern!(read::U56, u64, 7, |b: &[u8]| NativeEndian::read_uint(b, 7));
impl_read_fixnum_pattern!(BE<read::U56>, u64, 7, |b: &[u8]| BigEndian::read_uint(b, 7));
impl_read_fixnum_pattern!(LE<read::U56>, u64, 7, |b: &[u8]| LittleEndian::read_uint(b, 7));
impl_read_fixnum_pattern!(read::I56, i64, 7, |b: &[u8]| NativeEndian::read_int(b, 7));
impl_read_fixnum_pattern!(BE<read::I56>, i64, 7, |b: &[u8]| BigEndian::read_int(b, 7));
impl_read_fixnum_pattern!(LE<read::I56>, i64, 7, |b: &[u8]| LittleEndian::read_int(b, 7));

impl_read_fixnum_pattern!(read::U64, u64, 8, |b: &[u8]| NativeEndian::read_u64(b));
impl_read_fixnum_pattern!(BE<read::U64>, u64, 8, |b: &[u8]| BigEndian::read_u64(b));
impl_read_fixnum_pattern!(LE<read::U64>, u64, 8, |b: &[u8]| LittleEndian::read_u64(b));
impl_read_fixnum_pattern!(read::I64, i64, 8, |b: &[u8]| NativeEndian::read_i64(b));
impl_read_fixnum_pattern!(BE<read::I64>, i64, 8, |b: &[u8]| BigEndian::read_i64(b));
impl_read_fixnum_pattern!(LE<read::I64>, i64, 8, |b: &[u8]| LittleEndian::read_i64(b));

impl_read_fixnum_pattern!(read::F32, f32, 4, |b: &[u8]| NativeEndian::read_f32(b));
impl_read_fixnum_pattern!(BE<read::F32>, f32, 4, |b: &[u8]| BigEndian::read_f32(b));
impl_read_fixnum_pattern!(LE<read::F32>, f32, 4, |b: &[u8]| LittleEndian::read_f32(b));
impl_read_fixnum_pattern!(read::F64, f64, 8, |b: &[u8]| NativeEndian::read_f64(b));
impl_read_fixnum_pattern!(BE<read::F64>, f64, 8, |b: &[u8]| BigEndian::read_f64(b));
impl_read_fixnum_pattern!(LE<read::F64>, f64, 8, |b: &[u8]| LittleEndian::read_f64(b));

/// A future which will determine whether
/// the stream `R` is reached to the "End-Of-Stream" state.
///
/// This future is generally created by invoking
/// `ReadFrom::read_from` method for `Eos` pattern.
///
/// # Example
///
/// ```
/// # extern crate futures;
/// # extern crate handy_async;
/// use handy_async::io::ReadFrom;
/// use handy_async::pattern::read::Eos;
/// use futures::Future;
///
/// # fn main() {
/// let (_, is_eos) = Eos.read_from(std::io::empty()).wait().unwrap();
/// assert_eq!(is_eos, Ok(()));
///
/// // If target stream still contains any data,
/// // the first byte of the data will be returned.
/// let (_, (_, is_eos)) = (vec![0; 3], Eos).read_from(&mut &[0, 1, 2, 3][..]).wait().unwrap();
/// assert_eq!(is_eos, Err(3));
/// # }
/// ```
pub struct ReadEos<R>(ReadExact<PatternReader<R>, [u8; 1]>);
impl<R: Read> Future for ReadEos<R> {
    type Item = (PatternReader<R>, std::result::Result<(), u8>);
    type Error = AsyncIoError<PatternReader<R>>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.poll() {
            Err(e) => {
                if e.error_ref().kind() == ErrorKind::UnexpectedEof {
                    let ((r, _), _) = e.unwrap();
                    Ok(Async::Ready((r, Ok(()))))
                } else {
                    Err(e.map_state(|(r, _)| r))
                }
            }
            Ok(Async::Ready((r, b))) => Ok(Async::Ready((r, Err(b[0])))),
            Ok(Async::NotReady) => Ok(Async::NotReady),
        }
    }
}
impl<R: Read> AsyncMatch<PatternReader<R>> for read::Eos {
    type Future = ReadEos<R>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        ReadEos(matcher.async_read_exact([0; 1]))
    }
}

/// A future which will read a line string.
///
/// A line is ended with a newline character `\n`.
/// The final line ending is optional.
///
/// This future is generally created by invoking
/// `ReadFrom::read_from` method for `Line` pattern.
///
/// # Example
///
/// ```
/// # extern crate futures;
/// # extern crate handy_async;
/// use std::io::ErrorKind;
/// use handy_async::io::ReadFrom;
/// use handy_async::pattern::read::Line;
/// use futures::Future;
///
/// # fn main() {
/// let input = &b"hello\nworld!"[..];
///
/// let (input, line) = Line.read_from(input).wait().unwrap();
/// assert_eq!(line, "hello\n");
///
/// let (input, line) = Line.read_from(input).wait().unwrap();
/// assert_eq!(line, "world!");
///
/// let e = Line.read_from(input).wait().err().unwrap();
/// assert_eq!(e.error_ref().kind(), ErrorKind::UnexpectedEof);
/// # }
/// ```
pub struct ReadLine<R>(Option<(PatternReader<R>, Vec<u8>)>);
impl<R: Read> Future for ReadLine<R> {
    type Item = (PatternReader<R>, String);
    type Error = AsyncIoError<PatternReader<R>>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (mut reader, mut buf) = self.0.take().expect("Cannot poll ReadLine twice");

        let mut byte = [0; 1];
        match reader.read(&mut byte) {
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    self.0 = Some((reader, buf));
                    Ok(Async::NotReady)
                } else {
                    Err(AsyncIoError::new(reader, e))
                }
            }
            Ok(0) if buf.is_empty() => {
                let e = Error::new(ErrorKind::UnexpectedEof, "Cannot read a line");
                Err(AsyncIoError::new(reader, e))
            }
            Ok(read_size) => {
                let newline = if read_size == 0 {
                    true
                } else {
                    let b = byte[0];
                    buf.push(b);
                    b == '\n' as u8
                };
                if newline {
                    match String::from_utf8(buf) {
                        Err(e) => {
                            let e = Error::new(ErrorKind::InvalidInput, Box::new(e));
                            Err(AsyncIoError::new(reader, e))
                        }
                        Ok(line) => Ok(Async::Ready((reader, line))),
                    }
                } else {
                    self.0 = Some((reader, buf));
                    self.poll()
                }
            }
        }
    }
}
impl<R: Read> AsyncMatch<PatternReader<R>> for read::Line {
    type Future = ReadLine<R>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        ReadLine(Some((matcher, Vec::new())))
    }
}

/// A future which continues reading until `F` returns `Ok(Some(T))` or `Err(..)`.
///
/// This future is generally created by invoking
/// `ReadFrom::read_from` method for `Until` pattern.
pub struct ReadUntil<R, F, T>
    where R: Read,
          F: Fn(&[u8], bool) -> Result<Option<T>>
{
    read: ReadBytes<PatternReader<R>, Window<Vec<u8>>>,
    pred: F,
    max_buffer_size: usize,
}
impl<R: Read, F, T> Future for ReadUntil<R, F, T>
    where F: Fn(&[u8], bool) -> Result<Option<T>>
{
    type Item = (PatternReader<R>, (Vec<u8>, T));
    type Error = AsyncIoError<PatternReader<R>>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((r, mut b, read_size)) = self.read
            .poll()
            .map_err(|e| e.map_state(|(r, _)| r))? {
            let is_eos = read_size == 0;
            b = b.skip(read_size);
            let total_read_size = b.start();
            match (self.pred)(&b.inner_ref()[0..total_read_size], is_eos) {
                Err(e) => Err(AsyncIoError::new(r, e)),
                Ok(Some(v)) => {
                    let mut b = b.into_inner();
                    b.truncate(total_read_size);
                    Ok(Async::Ready((r, (b, v))))
                }
                Ok(None) if is_eos => {
                    let e = Error::new(ErrorKind::UnexpectedEof, "Unexpected Eof");
                    Err(AsyncIoError::new(r, e))
                }
                Ok(None) => {
                    if b.as_ref().is_empty() {
                        use std::cmp;
                        let new_len = cmp::min(total_read_size * 2, self.max_buffer_size);
                        let mut inner = b.into_inner();
                        if new_len == inner.len() {
                            let message = format!("Buffer size limit ({} bytes) reached",
                                                  self.max_buffer_size);
                            return Err(AsyncIoError::new(r, Error::new(ErrorKind::Other, message)));
                        }
                        inner.resize(total_read_size * 2, 0);
                        b = Window::new(inner).skip(total_read_size);
                    }
                    self.read = r.async_read(b);
                    self.poll()
                }
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}
impl<R: Read, F, T> AsyncMatch<PatternReader<R>> for read::Until<F, T>
    where F: Fn(&[u8], bool) -> Result<Option<T>>
{
    type Future = ReadUntil<R, F, T>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        let (pred, min_buffer_size, max_buffer_size) = self.unwrap();
        let buf = vec![0; min_buffer_size];
        ReadUntil {
            read: matcher.async_read(Window::new(buf)),
            pred: pred,
            max_buffer_size: max_buffer_size,
        }
    }
}

#[cfg(test)]
mod test {
    use std::io;
    use futures::Future;

    use pattern::{self, Pattern};
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(().and_then(|_| ())
                       .map(|_| 10)
                   .read_from(io::Cursor::new(vec![]))
                       .wait()
                       .unwrap()
                       .1,
                   10);

        let pattern = pattern::Iter(vec![(), (), ()].into_iter()).fold(0, |n, ()| n + 1);
        assert_eq!(pattern.read_from(io::Cursor::new(vec![])).wait().unwrap().1,
                   3);
    }
}
