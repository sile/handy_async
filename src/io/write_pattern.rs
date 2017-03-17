use std::io::{Write, Result, Error};
use futures::{Poll, Future};
use byteorder::{ByteOrder, NativeEndian, BigEndian, LittleEndian};

use pattern::{Buf, Window};
use pattern::write::{self, U24, I24, U40, I40, U48, I48, U56, I56};
use pattern::combinators::{PartialBuf, LE, BE};
use matcher::{AsyncMatch, Matcher};
use io::{AsyncWrite, AsyncIoError};

/// A matcher to write patterns into the inner writer `W`.
///
/// This is mainly used to define your own writing patterns.
/// See the example of the [WriteInto](./trait.WriteInto.html) trait.
pub struct PatternWriter<W>(W);
impl<W: Write> PatternWriter<W> {
    /// Makes new `PatternWriter` instance.
    pub fn new(inner: W) -> Self {
        PatternWriter(inner)
    }
}
impl<W: Write> Write for PatternWriter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        self.0.flush()
    }
}
impl<W> Matcher for PatternWriter<W> {
    type Error = Error;
}

/// The `WriteInto` trait allows for writing
/// a value of this pattern to a sink asynchronously.
///
/// # Notice
///
/// For executing asynchronously, we assume the writer `W` returns
/// `the std::io::ErrorKind::WouldBlock` error if a write operation would be about to block.
///
/// # Examples
///
/// Defines your own writing pattern:
///
/// ```
/// # extern crate futures;
/// # extern crate handy_async;
/// use std::io::Write;
/// use futures::{Future, BoxFuture};
/// use handy_async::io::{WriteInto, PatternWriter, AsyncIoError};
/// use handy_async::pattern::Pattern;
/// use handy_async::matcher::AsyncMatch;
///
/// // Defines pattern.
/// struct HelloWorld;
/// impl Pattern for HelloWorld {
///    type Value = ();
/// }
///
/// // Implements pattern maching between `PatternWriter<W>` and `HelloWorld`.
/// impl<W: Write + Send + 'static> AsyncMatch<PatternWriter<W>> for HelloWorld {
///     type Future = BoxFuture<(PatternWriter<W>, ()), AsyncIoError<PatternWriter<W>>>;
///     fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
///         Vec::from(&b"Hello World!"[..]).map(|_| ()).async_match(matcher).boxed()
///     }
/// }
///
/// # fn main() {
/// // Executes pattern matching.
/// let pattern = ("Hey! ".to_string(), HelloWorld);
/// let (output, _) = pattern.write_into(Vec::new()).wait().unwrap();
/// assert_eq!(output, b"Hey! Hello World!");
/// # }
/// ```
pub trait WriteInto<W: Write>: AsyncMatch<PatternWriter<W>> {
    /// Creates a future instance to write a value of the pattern to `writer`.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate futures;
    /// # extern crate handy_async;
    /// use futures::Future;
    /// use handy_async::io::WriteInto;
    /// use handy_async::pattern::Endian;
    ///
    /// # fn main() {
    /// let pattern = (1u8, 2u16.be());
    /// let (output, _) = pattern.write_into(Vec::new()).wait().unwrap();
    /// assert_eq!(output, [1, 0, 2]);
    /// # }
    /// ```
    fn write_into(self, writer: W) -> WritePattern<Self, W> {
        WritePattern(self.async_match(PatternWriter(writer)))
    }

    /// Synchronous version of the `WriteInto::write_into` method.
    fn sync_write_into(self, writer: W) -> Result<Self::Value> {
        self.write_into(writer)
            .wait()
            .map(|(_, v)| v)
            .map_err(|e| e.into_error())
    }
}
impl<W: Write, T> WriteInto<W> for T where T: AsyncMatch<PatternWriter<W>> {}

/// Future to write a pattern `P` into `W`.
///
/// This is created by calling `WriteInto::write_into` method.
pub struct WritePattern<P, W>(P::Future) where P: AsyncMatch<PatternWriter<W>>;
impl<P, W> Future for WritePattern<P, W>
    where P: AsyncMatch<PatternWriter<W>>
{
    type Item = (W, P::Value);
    type Error = AsyncIoError<W>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(self.0
               .poll()
               .map_err(|e| e.map_state(|w| w.0))?
               .map(|(m, v)| (m.0, v)))
    }
}

/// A future which will flush the internal buffer of `W`.
///
/// This is created by calling `WriteInto::write_into` method for
/// `Flush` pattern.
pub struct WriteFlush<W>(super::futures::Flush<PatternWriter<W>>);
impl<W: Write> Future for WriteFlush<W> {
    type Item = (PatternWriter<W>, ());
    type Error = AsyncIoError<PatternWriter<W>>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(self.0
               .poll()?
               .map(|m| (m, ())))
    }
}
impl<W: Write> AsyncMatch<PatternWriter<W>> for write::Flush {
    type Future = WriteFlush<W>;
    fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
        WriteFlush(matcher.async_flush())
    }
}

/// A future which will write bytes contained in the buffer `B` to `W`.
///
/// This future is generally created by invoking
/// `WriteInto::write_into` method for buffer like patterns
/// such as the following.
///
/// ```
/// use handy_async::io::WriteInto;
/// use handy_async::pattern::{Buf, Window};
///
/// vec![0; 32].write_into(std::io::sink());
/// Buf([0; 32]).write_into(std::io::sink());
/// Window::new([0; 32]).skip(4).write_into(std::io::sink());
/// ```
pub struct WriteBuf<W, B>(super::futures::WriteAll<PatternWriter<W>, B>);
impl<W: Write, B: AsRef<[u8]>> Future for WriteBuf<W, B> {
    type Item = (PatternWriter<W>, B);
    type Error = AsyncIoError<PatternWriter<W>>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll().map_err(|e| e.map_state(|(w, _)| w))
    }
}
impl<W: Write, B: AsRef<[u8]>> AsyncMatch<PatternWriter<W>> for Buf<B> {
    type Future = WriteBuf<W, B>;
    fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
        WriteBuf(matcher.async_write_all(self.0))
    }
}
impl<W: Write> AsyncMatch<PatternWriter<W>> for Vec<u8> {
    type Future = WriteBuf<W, Self>;
    fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
        WriteBuf(matcher.async_write_all(self))
    }
}
impl<W: Write> AsyncMatch<PatternWriter<W>> for String {
    type Future = WriteBuf<W, Self>;
    fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
        WriteBuf(matcher.async_write_all(self))
    }
}
impl<W: Write, B: AsRef<[u8]>> AsyncMatch<PatternWriter<W>> for Window<B> {
    type Future = WriteBuf<W, Self>;
    fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
        WriteBuf(matcher.async_write_all(self))
    }
}

/// A future which will write bytes contained in the buffer `B` to `W`
/// to the extent possible.
///
/// This future is generally created by invoking
/// `WriteInto::write_into` method for `PartialBuf` pattern
/// such as the following.
///
/// ```
/// # extern crate futures;
/// # extern crate handy_async;
/// use handy_async::io::WriteInto;
/// use handy_async::pattern::AllowPartial;
/// use futures::Future;
///
/// # fn main() {
/// // `PartialBuf` pattern is created via `allow_partial` method.
/// let pattern = vec![0; 32].allow_partial();
/// let (_, (_, written_size)) = pattern.write_into(&mut &mut [0; 4][..]).wait().unwrap();
/// assert_eq!(written_size, 4);
/// # }
/// ```
pub struct WritePartialBuf<W, B>(super::futures::WriteBytes<PatternWriter<W>, B>);
impl<W: Write, B: AsRef<[u8]>> Future for WritePartialBuf<W, B> {
    type Item = (PatternWriter<W>, (B, usize));
    type Error = AsyncIoError<PatternWriter<W>>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0
            .poll()
            .map(|a| a.map(|(w, b, s)| (w, (b, s))))
            .map_err(|e| e.map_state(|(w, _)| w))
    }
}
impl<W: Write, B: AsRef<[u8]>> AsyncMatch<PatternWriter<W>> for PartialBuf<B> {
    type Future = WritePartialBuf<W, B>;
    fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
        WritePartialBuf(matcher.async_write(self.0))
    }
}

/// A future which will write a fixnum associated with `P` into `W`.
pub struct WriteFixnum<W, P>
    where P: AsyncMatch<PatternWriter<W>>
{
    future: P::Future,
}
impl<W: Write, P> Future for WriteFixnum<W, P>
    where P: AsyncMatch<PatternWriter<W>>
{
    type Item = (PatternWriter<W>, ());
    type Error = AsyncIoError<PatternWriter<W>>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(self.future
               .poll()?
               .map(|(w, _)| (w, ())))
    }
}

macro_rules! impl_write_fixnum_pattern {
    ($pat:ty, $size:expr, $conv:expr) => {
        impl<W: Write> AsyncMatch<PatternWriter<W>> for $pat {
            type Future = WriteFixnum<W, Buf<[u8; $size]>>;
            fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
                let mut buf = [0; $size];
                $conv(&mut buf[..], self);
                let future = Buf(buf).async_match(matcher);
                WriteFixnum{future: future}
            }
        }
    }
}
impl_write_fixnum_pattern!(u8, 1, |b: &mut [u8], n: Self| b[0] = n);
impl_write_fixnum_pattern!(i8, 1, |b: &mut [u8], n: Self| b[0] = n as u8);

impl_write_fixnum_pattern!(u16, 2, NativeEndian::write_u16);
impl_write_fixnum_pattern!(BE<u16>, 2, |b: &mut [u8], n: Self| BigEndian::write_u16(b,n.0));
impl_write_fixnum_pattern!(LE<u16>, 2, |b: &mut [u8], n: Self| LittleEndian::write_u16(b,n.0));
impl_write_fixnum_pattern!(i16, 2, NativeEndian::write_i16);
impl_write_fixnum_pattern!(BE<i16>, 2, |b: &mut [u8], n: Self| BigEndian::write_i16(b,n.0));
impl_write_fixnum_pattern!(LE<i16>, 2, |b: &mut [u8], n: Self| LittleEndian::write_i16(b,n.0));

impl_write_fixnum_pattern!(U24, 3,
                           |b: &mut [u8], n: Self| NativeEndian::write_uint(b, n.0 as u64, 3));
impl_write_fixnum_pattern!(BE<U24>, 3,
                           |b: &mut [u8], n: Self| BigEndian::write_uint(b,(n.0).0 as u64, 3));
impl_write_fixnum_pattern!(LE<U24>, 3,
                           |b: &mut [u8], n: Self| LittleEndian::write_uint(b,(n.0).0 as u64, 3));
impl_write_fixnum_pattern!(I24, 3,
                           |b: &mut [u8], n: Self| NativeEndian::write_int(b, n.0 as i64, 3));
impl_write_fixnum_pattern!(BE<I24>, 3,
                           |b: &mut [u8], n: Self| BigEndian::write_int(b,(n.0).0 as i64, 3));
impl_write_fixnum_pattern!(LE<I24>, 3,
                           |b: &mut [u8], n: Self| LittleEndian::write_int(b,(n.0).0 as i64, 3));

impl_write_fixnum_pattern!(u32, 4, NativeEndian::write_u32);
impl_write_fixnum_pattern!(BE<u32>, 4, |b: &mut [u8], n: Self| BigEndian::write_u32(b,n.0));
impl_write_fixnum_pattern!(LE<u32>, 4, |b: &mut [u8], n: Self| LittleEndian::write_u32(b,n.0));
impl_write_fixnum_pattern!(i32, 4, NativeEndian::write_i32);
impl_write_fixnum_pattern!(BE<i32>, 4, |b: &mut [u8], n: Self| BigEndian::write_i32(b,n.0));
impl_write_fixnum_pattern!(LE<i32>, 4, |b: &mut [u8], n: Self| LittleEndian::write_i32(b,n.0));

impl_write_fixnum_pattern!(U40, 5,
                           |b: &mut [u8], n: Self| NativeEndian::write_uint(b, n.0 as u64, 5));
impl_write_fixnum_pattern!(BE<U40>, 5,
                           |b: &mut [u8], n: Self| BigEndian::write_uint(b,(n.0).0 as u64, 5));
impl_write_fixnum_pattern!(LE<U40>, 5,
                           |b: &mut [u8], n: Self| LittleEndian::write_uint(b,(n.0).0 as u64, 5));
impl_write_fixnum_pattern!(I40, 5,
                           |b: &mut [u8], n: Self| NativeEndian::write_int(b, n.0 as i64, 5));
impl_write_fixnum_pattern!(BE<I40>, 5,
                           |b: &mut [u8], n: Self| BigEndian::write_int(b,(n.0).0 as i64, 5));
impl_write_fixnum_pattern!(LE<I40>, 5,
                           |b: &mut [u8], n: Self| LittleEndian::write_int(b,(n.0).0 as i64, 5));

impl_write_fixnum_pattern!(U48, 6,
                           |b: &mut [u8], n: Self| NativeEndian::write_uint(b, n.0 as u64, 6));
impl_write_fixnum_pattern!(BE<U48>, 6,
                           |b: &mut [u8], n: Self| BigEndian::write_uint(b,(n.0).0 as u64, 6));
impl_write_fixnum_pattern!(LE<U48>, 6,
                           |b: &mut [u8], n: Self| LittleEndian::write_uint(b,(n.0).0 as u64, 6));
impl_write_fixnum_pattern!(I48, 6,
                           |b: &mut [u8], n: Self| NativeEndian::write_int(b, n.0 as i64, 6));
impl_write_fixnum_pattern!(BE<I48>, 6,
                           |b: &mut [u8], n: Self| BigEndian::write_int(b,(n.0).0 as i64, 6));
impl_write_fixnum_pattern!(LE<I48>, 6,
                           |b: &mut [u8], n: Self| LittleEndian::write_int(b,(n.0).0 as i64, 6));

impl_write_fixnum_pattern!(U56, 7,
                           |b: &mut [u8], n: Self| NativeEndian::write_uint(b, n.0 as u64, 7));
impl_write_fixnum_pattern!(BE<U56>, 7,
                           |b: &mut [u8], n: Self| BigEndian::write_uint(b,(n.0).0 as u64, 7));
impl_write_fixnum_pattern!(LE<U56>, 7,
                           |b: &mut [u8], n: Self| LittleEndian::write_uint(b,(n.0).0 as u64, 7));
impl_write_fixnum_pattern!(I56, 7,
                           |b: &mut [u8], n: Self| NativeEndian::write_int(b, n.0 as i64, 7));
impl_write_fixnum_pattern!(BE<I56>, 7,
                           |b: &mut [u8], n: Self| BigEndian::write_int(b,(n.0).0 as i64, 7));
impl_write_fixnum_pattern!(LE<I56>, 7,
                           |b: &mut [u8], n: Self| LittleEndian::write_int(b,(n.0).0 as i64, 7));

impl_write_fixnum_pattern!(u64, 8, NativeEndian::write_u64);
impl_write_fixnum_pattern!(BE<u64>, 8, |b: &mut [u8], n: Self| BigEndian::write_u64(b,n.0));
impl_write_fixnum_pattern!(LE<u64>, 8, |b: &mut [u8], n: Self| LittleEndian::write_u64(b,n.0));
impl_write_fixnum_pattern!(i64, 8, NativeEndian::write_i64);
impl_write_fixnum_pattern!(BE<i64>, 8, |b: &mut [u8], n: Self| BigEndian::write_i64(b,n.0));
impl_write_fixnum_pattern!(LE<i64>, 8, |b: &mut [u8], n: Self| LittleEndian::write_i64(b,n.0));
