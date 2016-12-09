use std::io::{Write, Result, Error};
use futures::{Poll, Future};
use byteorder::{ByteOrder, NativeEndian, BigEndian, LittleEndian};

use pattern::{Pattern, AsyncMatch, Buf, Window};
use pattern::write::{self, U24, I24, U40, I40, U48, I48, U56, I56};
use pattern::combinators::{self, PartialBuf, LE, BE};
use io::{AsyncWrite, AsyncError};

pub struct PatternWriter<W>(W);
impl<W> PatternWriter<W> {
    pub fn into_inner(self) -> W {
        self.0
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

/// The `WritePattern` trait allows for writing
/// a value of the pattern to a sink asynchronously.
///
/// # Notice
///
/// For executing asynchronously, we assume the writer `W` returns
/// `the std::io::ErrorKind::WouldBlock` error if a write operation would be about to block.
///
/// # Examples
///
/// Defines original pattern:
///
/// ```
/// # extern crate futures;
/// # extern crate handy_io;
/// use std::io::{Write, Error};
/// use futures::{Poll, Future};
/// use handy_io::io::{WritePattern, PatternWriter};
/// use handy_io::io::futures::WriteBuf;
/// use handy_io::pattern::{Pattern, AsyncMatch};
///
/// struct WriteHelloWorld<W>(WriteBuf<W, Vec<u8>>);
/// impl<W: Write> Future for WriteHelloWorld<W> {
///     type Item = (PatternWriter<W>, ());
///     type Error = (PatternWriter<W>, Error);
///     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
///         Ok(self.0.poll()?.map(|(w, _)| (w, ())))
///     }
/// }
///
/// struct HelloWorld;
/// impl Pattern for HelloWorld {
///    type Value = ();
/// }
/// impl<W: Write> AsyncMatch<PatternWriter<W>, Error> for HelloWorld {
///     type Future = WriteHelloWorld<W>;
///     fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
///         WriteHelloWorld(Vec::from(&b"Hello World!"[..]).async_match(matcher))
///     }
/// }
///
/// # fn main() {
/// let pattern = ("Hey! ".to_string(), HelloWorld);
/// let (output, _) = pattern.write_to(Vec::new()).wait().unwrap();
/// assert_eq!(output, b"Hey! Hello World!");
/// # }
/// ```
pub trait WritePattern<W: Write>: AsyncMatch<PatternWriter<W>, Error> {
    /// Creates a future instance to write a value of the pattern to `writer`.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate futures;
    /// # extern crate handy_io;
    /// use futures::Future;
    /// use handy_io::io::WritePattern;
    /// use handy_io::pattern::Endian;
    ///
    /// # fn main() {
    /// let pattern = (1u8, 2u16.be());
    /// let (output, _) = pattern.write_to(Vec::new()).wait().unwrap();
    /// assert_eq!(output, [1, 0, 2]);
    /// # }
    /// ```
    fn write_to(self, writer: W) -> WriteTo<Self, W> {
        WriteTo(self.async_match(PatternWriter(writer)))
    }
}
impl<W: Write, T> WritePattern<W> for T where T: AsyncMatch<PatternWriter<W>, Error> {}

pub struct WriteTo<P, W>(P::Future) where P: AsyncMatch<PatternWriter<W>, Error>;
impl<P, W> Future for WriteTo<P, W>
    where P: AsyncMatch<PatternWriter<W>, Error>
{
    type Item = (W, P::Value);
    type Error = AsyncError<W>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(self.0.poll().map_err(|(m, e)| AsyncError::new(m.0, e))?.map(|(m, v)| (m.0, v)))
    }
}

pub struct WriteFlush<W>(super::futures::Flush<PatternWriter<W>>);
impl<W: Write> Future for WriteFlush<W> {
    type Item = (PatternWriter<W>, ());
    type Error = (PatternWriter<W>, Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(self.0.poll().map_err(|e| e.unwrap())?.map(|m| (m, ())))
    }
}
impl<W: Write> AsyncMatch<PatternWriter<W>, Error> for write::Flush {
    type Future = WriteFlush<W>;
    fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
        WriteFlush(matcher.async_flush())
    }
}

pub struct WriteBuf<W, B>(super::futures::WriteAll<PatternWriter<W>, B>);
impl<W: Write, B: AsRef<[u8]>> Future for WriteBuf<W, B> {
    type Item = (PatternWriter<W>, B);
    type Error = (PatternWriter<W>, Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll().map_err(|e| e.map(|(m, _)| m).unwrap())
    }
}
impl<W: Write, B: AsRef<[u8]>> AsyncMatch<PatternWriter<W>, Error> for Buf<B> {
    type Future = WriteBuf<W, B>;
    fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
        WriteBuf(matcher.async_write_all(self.0))
    }
}
impl<W: Write> AsyncMatch<PatternWriter<W>, Error> for Vec<u8> {
    type Future = WriteBuf<W, Self>;
    fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
        WriteBuf(matcher.async_write_all(self))
    }
}
impl<W: Write> AsyncMatch<PatternWriter<W>, Error> for String {
    type Future = WriteBuf<W, Self>;
    fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
        WriteBuf(matcher.async_write_all(self))
    }
}
impl<W: Write, B: AsRef<[u8]>> AsyncMatch<PatternWriter<W>, Error> for Window<B> {
    type Future = WriteBuf<W, Self>;
    fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
        WriteBuf(matcher.async_write_all(self))
    }
}

pub struct WritePartialBuf<W, B>(super::futures::WriteBytes<PatternWriter<W>, B>);
impl<W: Write, B: AsRef<[u8]>> Future for WritePartialBuf<W, B> {
    type Item = (PatternWriter<W>, (B, usize));
    type Error = (PatternWriter<W>, Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0
            .poll()
            .map(|a| a.map(|(w, b, s)| (w, (b, s))))
            .map_err(|e| e.map(|(w, _)| w).unwrap())
    }
}
impl<W: Write, B: AsRef<[u8]>> AsyncMatch<PatternWriter<W>, Error> for PartialBuf<B> {
    type Future = WritePartialBuf<W, B>;
    fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
        WritePartialBuf(matcher.async_write(self.0))
    }
}

pub type WriteFixnum<W, P, T> where P: Pattern =
    <combinators::Map<P, fn (P::Value) -> T> as AsyncMatch<PatternWriter<W>, Error>>::Future;
macro_rules! impl_write_fixnum_pattern {
    ($pat:ty, $size:expr, $conv:expr) => {
        impl<W: Write> AsyncMatch<PatternWriter<W>, Error> for $pat {
            type Future = WriteFixnum<W, Buf<[u8; $size]>, ()>;
            fn async_match(self, matcher: PatternWriter<W>) -> Self::Future {
                fn null(_: [u8; $size]) -> () { ()}
                let mut buf = [0; $size];
                $conv(&mut buf[..], self);
                Buf(buf).map(null as _).async_match(matcher)
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
