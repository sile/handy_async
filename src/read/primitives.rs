use std::io::{self, Read};
use futures::{self, Poll, Future};
use byteorder::{ByteOrder, NativeEndian, LittleEndian, BigEndian};

use pattern::{self, Window, Buf, Pattern, BE, LE};
use pattern::read;
use super::{ReadFrom, AsyncRead, ReadExact, ReadFold};

pub type MapFuture<R, P, T> where P: Pattern =
    <pattern::Map<P, fn(P::Value) -> T> as ReadFrom<R>>::Future;

pub type AndThenFuture<R, P, T> where P: Pattern =
    <pattern::AndThen<P, fn(P::Value) -> io::Result<T>> as ReadFrom<R>>::Future;

pub type ThenFuture<R, P, T> where P: Pattern =
    <pattern::Then<P, fn(io::Result<P::Value>) -> io::Result<T>> as ReadFrom<R>>::Future;

pub struct ReadBuf<R, B>(ReadExact<R, B>);
impl<R: Read, B: AsMut<[u8]>> Future for ReadBuf<R, B> {
    type Item = (R, B);
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll().map_err(|(r, _, e)| (r, e))
    }
}

impl<R: Read> ReadFrom<R> for Vec<u8> {
    type Future = ReadBuf<R, Self>;
    fn read_from(self, reader: R) -> Self::Future {
        ReadBuf(reader.async_read_exact(self))
    }
}

impl<R: Read> ReadFrom<R> for String {
    type Future = AndThenFuture<R, Vec<u8>, String>;
    fn read_from(self, reader: R) -> Self::Future {
        fn to_str(bytes: Vec<u8>) -> io::Result<String> {
            String::from_utf8(bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, Box::new(e)))
        }
        self.into_bytes().and_then(to_str as _).read_from(reader)
    }
}

impl<R: Read, B: AsMut<[u8]>> ReadFrom<R> for Window<B> {
    type Future = ReadBuf<R, Self>;
    fn read_from(self, reader: R) -> Self::Future {
        ReadBuf(reader.async_read_exact(self))
    }
}

impl<R: Read, B: AsMut<[u8]>> ReadFrom<R> for Buf<B> {
    type Future = futures::Map<ReadBuf<R, Self>, fn((R, Buf<B>)) -> (R, B)>;
    fn read_from(self, reader: R) -> Self::Future {
        fn unwrap<R, B>((r, b): (R, Buf<B>)) -> (R, B) {
            (r, b.0)
        }
        ReadBuf(reader.async_read_exact(self)).map(unwrap as _)
    }
}

pub struct ReadUntil<R, F>(ReadFold<R,
                                    fn(F, Window<Vec<u8>>, usize)
                                       -> Result<(Window<Vec<u8>>, F),
                                                  (Window<Vec<u8>>, io::Result<F>)>,
                                    Window<Vec<u8>>,
                                    F>);
impl<R: Read, F> Future for ReadUntil<R, F>
    where F: for<'a> Fn(&'a [u8], usize) -> bool
{
    type Item = (R, Vec<u8>);
    type Error = (R, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(self.0.poll().map_err(|(r, _, e)| (r, e))?.map(|(r, b, _)| {
            let total_read_size = b.start();
            let mut inner = b.into_inner();
            inner.truncate(total_read_size);
            (r, inner)
        }))
    }
}
impl<R: Read, F> ReadFrom<R> for read::Until<F>
    where F: for<'a> Fn(&'a [u8], usize) -> bool
{
    type Future = ReadUntil<R, F>;
    fn read_from(self, reader: R) -> Self::Future {
        fn fold<F>(f: F,
                   buf: Window<Vec<u8>>,
                   read_size: usize)
                   -> Result<(Window<Vec<u8>>, F), (Window<Vec<u8>>, io::Result<F>)>
            where F: for<'a> Fn(&'a [u8], usize) -> bool
        {
            let offset = buf.start();
            let buf = buf.skip(read_size);
            let total_read_size = buf.start();
            if f(&buf.inner_ref()[..total_read_size], offset) {
                Err((buf, Ok(f)))
            } else if read_size == 0 {
                Err((buf,
                     Err(io::Error::new(io::ErrorKind::UnexpectedEof,
                                        "Unexpected Eof in ReadUntil"))))
            } else {
                let buf = if buf.as_ref().is_empty() {
                    let mut inner = buf.into_inner();
                    inner.resize(total_read_size + 1024, 0);
                    Window::new(inner).skip(total_read_size)
                } else {
                    buf
                };
                Ok((buf, f))
            }
        }
        ReadUntil(reader.async_read_fold(Window::new(vec![0; 1024]), self.0, fold as _))
    }
}

macro_rules! impl_fixnum_read_from {
    ($pat:ty, $val:ident, $size:expr, $conv:expr) => {
        impl<R: Read> ReadFrom<R> for $pat {
            type Future = MapFuture<R, Buf<[u8; $size]>, Self::Value>;
            fn read_from(self, reader: R) -> Self::Future {
                fn conv(b: [u8; $size]) -> $val {
                    $conv(&b[..]) as $val
                }
                Buf([0; $size]).map(conv as _).read_from(reader)
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

impl<R: Read> ReadFrom<R> for read::Eos {
    type Future = ThenFuture<R, read::U8, Self::Value>;
    fn read_from(self, reader: R) -> Self::Future {
        fn to_eos(r: io::Result<u8>) -> io::Result<Result<(), u8>> {
            match r {
                Ok(b) => Ok(Err(b)),
                Err(e) => {
                    if e.kind() == io::ErrorKind::UnexpectedEof {
                        Ok(Ok(()))
                    } else {
                        Err(e)
                    }
                }
            }
        }
        read::U8.then(to_eos as _).read_from(reader)
    }
}
