use std::io::{self, Read};
use futures::{self, Poll, Future};
use byteorder::{ByteOrder, NativeEndian, LittleEndian, BigEndian};

use pattern::{self, Window, Buf, Pattern};
use pattern::combinators::{BE, LE, PartialBuf};
use pattern::read;
use super::{ReadFrom, AsyncRead, ReadExact, ReadNonEmpty};
use io::futures as io_futures;

/// A future which will read bytes from `R` to fill the buffer `B` completely.
///
/// This future is generally created by invoking
/// `ReadFrom::read_from` method for buffer like patterns
/// such as the following.
///
/// ```
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::{Buf, Window};
///
/// vec![0; 32].read_from(std::io::empty());
/// Buf([0; 32]).read_from(std::io::empty());
/// Window::new([0; 32]).skip(4).read_from(std::io::empty());
/// ```
pub struct ReadBuf<R, B>(ReadExact<R, B>);
impl<R: Read, B: AsMut<[u8]>> Future for ReadBuf<R, B> {
    type Item = (R, B);
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll().map_err(|(r, _, e)| (r, e))
    }
}
impl<R: Read, B: AsMut<[u8]>> ReadFrom<R> for Buf<B> {
    type Future = futures::Map<ReadBuf<R, Self>, fn((R, Buf<B>)) -> (R, B)>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        fn unwrap<R, B>((r, b): (R, Buf<B>)) -> (R, B) {
            (r, b.0)
        }
        ReadBuf(reader.async_read_exact(self)).map(unwrap as _)
    }
}
impl<R: Read> ReadFrom<R> for Vec<u8> {
    type Future = ReadBuf<R, Self>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        ReadBuf(reader.async_read_exact(self))
    }
}
impl<R: Read, B: AsMut<[u8]>> ReadFrom<R> for Window<B> {
    type Future = ReadBuf<R, Self>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        ReadBuf(reader.async_read_exact(self))
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
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::AllowPartial;
///
/// // `PartialBuf` pattern is created via `allow_partial` method.
/// let pattern = vec![0; 32].allow_partial();
/// let (_, read_size) = pattern.sync_read_from(&mut &[0; 4][..]).unwrap();
/// assert_eq!(read_size, 4);
/// ```
pub struct ReadPartialBuf<R, B>(ReadNonEmpty<R, B>);
impl<R: Read, B: AsMut<[u8]>> Future for ReadPartialBuf<R, B> {
    type Item = (R, (B, usize));
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll().map(|x| x.map(|(r, b, s)| (r, (b, s)))).map_err(|(r, _, e)| (r, e))
    }
}
impl<R: Read, B: AsMut<[u8]>> ReadFrom<R> for PartialBuf<B> {
    type Future = ReadPartialBuf<R, B>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        ReadPartialBuf(reader.async_read_non_empty(self.0))
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
/// use handy_io::io::ReadFrom;
///
/// let str_buf = String::from_utf8(vec![0; 32]).unwrap();
/// str_buf.read_from(std::io::empty());
/// ```
pub type ReadString<R> = io_futures::ReadAndThen<R,
                                                 Vec<u8>,
                                                 io::Result<String>,
                                                 fn(Vec<u8>) -> io::Result<String>>;
impl<R: Read> ReadFrom<R> for String {
    type Future = ReadString<R>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        fn to_str(bytes: Vec<u8>) -> io::Result<String> {
            String::from_utf8(bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, Box::new(e)))
        }
        self.into_bytes().and_then(to_str as _).lossless_read_from(reader)
    }
}

type MapFuture<R, P, T> where P: Pattern =
    <pattern::combinators::Map<P, fn(P::Value) -> T> as ReadFrom<R>>::Future;
macro_rules! impl_fixnum_read_from {
    ($pat:ty, $val:ident, $size:expr, $conv:expr) => {
        impl<R: Read> ReadFrom<R> for $pat {
            type Future = MapFuture<R, Buf<[u8; $size]>, Self::Value>;
            fn lossless_read_from(self, reader: R) -> Self::Future {
                fn conv(b: [u8; $size]) -> $val {
                    $conv(&b[..]) as $val
                }
                Buf([0; $size]).map(conv as _).lossless_read_from(reader)
            }
        }
    }
}
impl_fixnum_read_from!(read::U8, u8, 1, |b: &[u8]| b[0]);
impl_fixnum_read_from!(read::I8, i8, 1, |b: &[u8]| b[0] as i8);

impl_fixnum_read_from!(read::U16, u16, 2, |b: &[u8]| NativeEndian::read_u16(b));
impl_fixnum_read_from!(BE<read::U16>, u16, 2, |b: &[u8]| BigEndian::read_u16(b));
impl_fixnum_read_from!(LE<read::U16>, u16, 2, |b: &[u8]| LittleEndian::read_u16(b));
impl_fixnum_read_from!(read::I16, i16, 2, |b: &[u8]| NativeEndian::read_i16(b));
impl_fixnum_read_from!(BE<read::I16>, i16, 2, |b: &[u8]| BigEndian::read_i16(b));
impl_fixnum_read_from!(LE<read::I16>, i16, 2, |b: &[u8]| LittleEndian::read_i16(b));

impl_fixnum_read_from!(read::U24, u32, 3, |b: &[u8]| NativeEndian::read_uint(b, 3));
impl_fixnum_read_from!(BE<read::U24>, u32, 3, |b: &[u8]| BigEndian::read_uint(b, 3));
impl_fixnum_read_from!(LE<read::U24>, u32, 3, |b: &[u8]| LittleEndian::read_uint(b, 3));
impl_fixnum_read_from!(read::I24, i32, 3, |b: &[u8]| NativeEndian::read_int(b, 3));
impl_fixnum_read_from!(BE<read::I24>, i32, 3, |b: &[u8]| BigEndian::read_int(b, 3));
impl_fixnum_read_from!(LE<read::I24>, i32, 3, |b: &[u8]| LittleEndian::read_int(b, 3));

impl_fixnum_read_from!(read::U32, u32, 4, |b: &[u8]| NativeEndian::read_u32(b));
impl_fixnum_read_from!(BE<read::U32>, u32, 4, |b: &[u8]| BigEndian::read_u32(b));
impl_fixnum_read_from!(LE<read::U32>, u32, 4, |b: &[u8]| LittleEndian::read_u32(b));
impl_fixnum_read_from!(read::I32, i32, 4, |b: &[u8]| NativeEndian::read_i32(b));
impl_fixnum_read_from!(BE<read::I32>, i32, 4, |b: &[u8]| BigEndian::read_i32(b));
impl_fixnum_read_from!(LE<read::I32>, i32, 4, |b: &[u8]| LittleEndian::read_i32(b));

impl_fixnum_read_from!(read::U40, u64, 5, |b: &[u8]| NativeEndian::read_uint(b, 5));
impl_fixnum_read_from!(BE<read::U40>, u64, 5, |b: &[u8]| BigEndian::read_uint(b, 5));
impl_fixnum_read_from!(LE<read::U40>, u64, 5, |b: &[u8]| LittleEndian::read_uint(b, 5));
impl_fixnum_read_from!(read::I40, i64, 5, |b: &[u8]| NativeEndian::read_int(b, 5));
impl_fixnum_read_from!(BE<read::I40>, i64, 5, |b: &[u8]| BigEndian::read_int(b, 5));
impl_fixnum_read_from!(LE<read::I40>, i64, 5, |b: &[u8]| LittleEndian::read_int(b, 5));

impl_fixnum_read_from!(read::U48, u64, 6, |b: &[u8]| NativeEndian::read_uint(b, 6));
impl_fixnum_read_from!(BE<read::U48>, u64, 6, |b: &[u8]| BigEndian::read_uint(b, 6));
impl_fixnum_read_from!(LE<read::U48>, u64, 6, |b: &[u8]| LittleEndian::read_uint(b, 6));
impl_fixnum_read_from!(read::I48, i64, 6, |b: &[u8]| NativeEndian::read_int(b, 6));
impl_fixnum_read_from!(BE<read::I48>, i64, 6, |b: &[u8]| BigEndian::read_int(b, 6));
impl_fixnum_read_from!(LE<read::I48>, i64, 6, |b: &[u8]| LittleEndian::read_int(b, 6));

impl_fixnum_read_from!(read::U56, u64, 7, |b: &[u8]| NativeEndian::read_uint(b, 7));
impl_fixnum_read_from!(BE<read::U56>, u64, 7, |b: &[u8]| BigEndian::read_uint(b, 7));
impl_fixnum_read_from!(LE<read::U56>, u64, 7, |b: &[u8]| LittleEndian::read_uint(b, 7));
impl_fixnum_read_from!(read::I56, i64, 7, |b: &[u8]| NativeEndian::read_int(b, 7));
impl_fixnum_read_from!(BE<read::I56>, i64, 7, |b: &[u8]| BigEndian::read_int(b, 7));
impl_fixnum_read_from!(LE<read::I56>, i64, 7, |b: &[u8]| LittleEndian::read_int(b, 7));

impl_fixnum_read_from!(read::U64, u64, 8, |b: &[u8]| NativeEndian::read_u64(b));
impl_fixnum_read_from!(BE<read::U64>, u64, 8, |b: &[u8]| BigEndian::read_u64(b));
impl_fixnum_read_from!(LE<read::U64>, u64, 8, |b: &[u8]| LittleEndian::read_u64(b));
impl_fixnum_read_from!(read::I64, i64, 8, |b: &[u8]| NativeEndian::read_i64(b));
impl_fixnum_read_from!(BE<read::I64>, i64, 8, |b: &[u8]| BigEndian::read_i64(b));
impl_fixnum_read_from!(LE<read::I64>, i64, 8, |b: &[u8]| LittleEndian::read_i64(b));

impl_fixnum_read_from!(read::F32, f32, 4, |b: &[u8]| NativeEndian::read_f32(b));
impl_fixnum_read_from!(BE<read::F32>, f32, 4, |b: &[u8]| BigEndian::read_f32(b));
impl_fixnum_read_from!(LE<read::F32>, f32, 4, |b: &[u8]| LittleEndian::read_f32(b));
impl_fixnum_read_from!(read::F64, f64, 8, |b: &[u8]| NativeEndian::read_f64(b));
impl_fixnum_read_from!(BE<read::F64>, f64, 8, |b: &[u8]| BigEndian::read_f64(b));
impl_fixnum_read_from!(LE<read::F64>, f64, 8, |b: &[u8]| LittleEndian::read_f64(b));

/// A future which will determine whether
/// the stream `R` is reached to the "End-Of-Stream" state.
///
/// This future is generally created by invoking
/// `ReadFrom::read_from` method for `Eos` pattern.
///
/// # Example
///
/// ```
/// use handy_io::io::ReadFrom;
/// use handy_io::pattern::read::Eos;
///
/// let is_eos = Eos.sync_read_from(std::io::empty()).unwrap();
/// assert_eq!(is_eos, Ok(()));
///
/// // If target stream still contains any data,
/// // the first byte of the data will be returned.
/// let (_, is_eos) = (vec![0; 3], Eos).sync_read_from(&mut &[0, 1, 2, 3][..]).unwrap();
/// assert_eq!(is_eos, Err(3));
/// ```
pub type ReadEos<R> where R: Read = io_futures::ReadThen<R,
                                                         read::U8,
                                                         io::Result<Result<(), u8>>,
                                                         fn(io::Result<u8>)
                                                            -> io::Result<Result<(), u8>>>;
impl<R: Read> ReadFrom<R> for read::Eos {
    type Future = ReadEos<R>;
    fn lossless_read_from(self, reader: R) -> Self::Future {
        fn to_eos(r: io::Result<u8>) -> io::Result<Result<(), u8>> {
            r.map(Err).or_else(|e| {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    Ok(Ok(()))
                } else {
                    Err(e)
                }
            })
        }
        read::U8.then(to_eos as _).lossless_read_from(reader)
    }
}
