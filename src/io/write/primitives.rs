use std::io::{self, Write};
use futures::{self, Poll, Future};
use byteorder::{ByteOrder, NativeEndian, BigEndian, LittleEndian};

use pattern::{self, Pattern};
use pattern::combinators::{LE, BE, PartialBuf};
use pattern::write::{self, U24, U40, U48, U56, I24, I40, I48, I56};
use super::{AsyncWrite, WriteTo, WriteBytes};

impl<W: Write> WriteTo<W> for write::Flush {
    type Future = futures::Map<super::Flush<W>, fn(W) -> (W, ())>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        fn conv<W>(w: W) -> (W, ()) {
            (w, ())
        }
        writer.async_flush().map(conv as _)
    }
}

pub type WriteBuf<W, B> = futures::MapErr<super::WriteAll<W, B>,
                                          fn((W, B, io::Error)) -> (W, io::Error)>;
impl<W: Write, B: AsRef<[u8]>> WriteTo<W> for pattern::Buf<B> {
    type Future = WriteBuf<W, B>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        fn conv<A, B, C>((a, _, c): (A, B, C)) -> (A, C) {
            (a, c)
        }
        writer.async_write_all(self.0).map_err(conv as _)
    }
}

pub struct WritePartialBuf<W, B>(WriteBytes<W, B>);
impl<W: Write, B: AsRef<[u8]>> Future for WritePartialBuf<W, B> {
    type Item = (W, (B, usize));
    type Error = (W, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll().map(|a| a.map(|(w, b, s)| (w, (b, s)))).map_err(|(w, _, e)| (w, e))
    }
}
impl<W: Write, B: AsRef<[u8]>> WriteTo<W> for PartialBuf<B> {
    type Future = WritePartialBuf<W, B>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        WritePartialBuf(writer.async_write(self.0))
    }
}

impl<W: Write> WriteTo<W> for Vec<u8> {
    type Future = <pattern::Buf<Self> as WriteTo<W>>::Future;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        pattern::Buf(self).lossless_write_to(writer)
    }
}

impl<W: Write> WriteTo<W> for String {
    type Future = MapFuture<W, Vec<u8>, String>;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        fn conv(v: Vec<u8>) -> String {
            unsafe { String::from_utf8_unchecked(v) }
        }
        self.into_bytes().map(conv as _).lossless_write_to(writer)
    }
}

impl<W: Write, B: AsRef<[u8]>> WriteTo<W> for pattern::Window<B> {
    type Future = <pattern::Buf<Self> as WriteTo<W>>::Future;
    fn lossless_write_to(self, writer: W) -> Self::Future {
        pattern::Buf(self).lossless_write_to(writer)
    }
}

type MapFuture<W,P,T> where P: WriteTo<W> =
     <pattern::combinators::Map<P, fn (P::Value) -> T> as WriteTo<W>>::Future;
macro_rules! impl_fixnum_write_to {
    ($pat:ty, $size:expr, $conv:expr) => {
        impl<W: Write> WriteTo<W> for $pat {
            type Future = MapFuture<W, pattern::Buf<[u8; $size]>, ()>;
            fn lossless_write_to(self, writer: W) -> Self::Future {
                fn null(_: [u8; $size]) -> () { ()}
                let mut buf = [0; $size];
                $conv(&mut buf[..], self);
                pattern::Buf(buf).map(null as _).lossless_write_to(writer)
            }
        }
    }
}
impl_fixnum_write_to!(u8, 1, |b: &mut [u8], n: Self| b[0] = n);
impl_fixnum_write_to!(i8, 1, |b: &mut [u8], n: Self| b[0] = n as u8);

impl_fixnum_write_to!(u16, 2, NativeEndian::write_u16);
impl_fixnum_write_to!(BE<u16>, 2, |b: &mut [u8], n: Self| BigEndian::write_u16(b,n.0));
impl_fixnum_write_to!(LE<u16>, 2, |b: &mut [u8], n: Self| LittleEndian::write_u16(b,n.0));
impl_fixnum_write_to!(i16, 2, NativeEndian::write_i16);
impl_fixnum_write_to!(BE<i16>, 2, |b: &mut [u8], n: Self| BigEndian::write_i16(b,n.0));
impl_fixnum_write_to!(LE<i16>, 2, |b: &mut [u8], n: Self| LittleEndian::write_i16(b,n.0));

impl_fixnum_write_to!(U24, 3, |b: &mut [u8], n: Self| NativeEndian::write_uint(b, n.0 as u64, 3));
impl_fixnum_write_to!(BE<U24>, 3,
                      |b: &mut [u8], n: Self| BigEndian::write_uint(b,(n.0).0 as u64, 3));
impl_fixnum_write_to!(LE<U24>, 3,
                      |b: &mut [u8], n: Self| LittleEndian::write_uint(b,(n.0).0 as u64, 3));
impl_fixnum_write_to!(I24, 3, |b: &mut [u8], n: Self| NativeEndian::write_int(b, n.0 as i64, 3));
impl_fixnum_write_to!(BE<I24>, 3,
                      |b: &mut [u8], n: Self| BigEndian::write_int(b,(n.0).0 as i64, 3));
impl_fixnum_write_to!(LE<I24>, 3,
                      |b: &mut [u8], n: Self| LittleEndian::write_int(b,(n.0).0 as i64, 3));

impl_fixnum_write_to!(u32, 4, NativeEndian::write_u32);
impl_fixnum_write_to!(BE<u32>, 4, |b: &mut [u8], n: Self| BigEndian::write_u32(b,n.0));
impl_fixnum_write_to!(LE<u32>, 4, |b: &mut [u8], n: Self| LittleEndian::write_u32(b,n.0));
impl_fixnum_write_to!(i32, 4, NativeEndian::write_i32);
impl_fixnum_write_to!(BE<i32>, 4, |b: &mut [u8], n: Self| BigEndian::write_i32(b,n.0));
impl_fixnum_write_to!(LE<i32>, 4, |b: &mut [u8], n: Self| LittleEndian::write_i32(b,n.0));

impl_fixnum_write_to!(U40, 5, |b: &mut [u8], n: Self| NativeEndian::write_uint(b, n.0 as u64, 5));
impl_fixnum_write_to!(BE<U40>, 5,
                      |b: &mut [u8], n: Self| BigEndian::write_uint(b,(n.0).0 as u64, 5));
impl_fixnum_write_to!(LE<U40>, 5,
                      |b: &mut [u8], n: Self| LittleEndian::write_uint(b,(n.0).0 as u64, 5));
impl_fixnum_write_to!(I40, 5, |b: &mut [u8], n: Self| NativeEndian::write_int(b, n.0 as i64, 5));
impl_fixnum_write_to!(BE<I40>, 5,
                      |b: &mut [u8], n: Self| BigEndian::write_int(b,(n.0).0 as i64, 5));
impl_fixnum_write_to!(LE<I40>, 5,
                      |b: &mut [u8], n: Self| LittleEndian::write_int(b,(n.0).0 as i64, 5));

impl_fixnum_write_to!(U48, 6, |b: &mut [u8], n: Self| NativeEndian::write_uint(b, n.0 as u64, 6));
impl_fixnum_write_to!(BE<U48>, 6,
                      |b: &mut [u8], n: Self| BigEndian::write_uint(b,(n.0).0 as u64, 6));
impl_fixnum_write_to!(LE<U48>, 6,
                      |b: &mut [u8], n: Self| LittleEndian::write_uint(b,(n.0).0 as u64, 6));
impl_fixnum_write_to!(I48, 6, |b: &mut [u8], n: Self| NativeEndian::write_int(b, n.0 as i64, 6));
impl_fixnum_write_to!(BE<I48>, 6,
                      |b: &mut [u8], n: Self| BigEndian::write_int(b,(n.0).0 as i64, 6));
impl_fixnum_write_to!(LE<I48>, 6,
                      |b: &mut [u8], n: Self| LittleEndian::write_int(b,(n.0).0 as i64, 6));

impl_fixnum_write_to!(U56, 7, |b: &mut [u8], n: Self| NativeEndian::write_uint(b, n.0 as u64, 7));
impl_fixnum_write_to!(BE<U56>, 7,
                      |b: &mut [u8], n: Self| BigEndian::write_uint(b,(n.0).0 as u64, 7));
impl_fixnum_write_to!(LE<U56>, 7,
                      |b: &mut [u8], n: Self| LittleEndian::write_uint(b,(n.0).0 as u64, 7));
impl_fixnum_write_to!(I56, 7, |b: &mut [u8], n: Self| NativeEndian::write_int(b, n.0 as i64, 7));
impl_fixnum_write_to!(BE<I56>, 7,
                      |b: &mut [u8], n: Self| BigEndian::write_int(b,(n.0).0 as i64, 7));
impl_fixnum_write_to!(LE<I56>, 7,
                      |b: &mut [u8], n: Self| LittleEndian::write_int(b,(n.0).0 as i64, 7));

impl_fixnum_write_to!(u64, 8, NativeEndian::write_u64);
impl_fixnum_write_to!(BE<u64>, 8, |b: &mut [u8], n: Self| BigEndian::write_u64(b,n.0));
impl_fixnum_write_to!(LE<u64>, 8, |b: &mut [u8], n: Self| LittleEndian::write_u64(b,n.0));
impl_fixnum_write_to!(i64, 8, NativeEndian::write_i64);
impl_fixnum_write_to!(BE<i64>, 8, |b: &mut [u8], n: Self| BigEndian::write_i64(b,n.0));
impl_fixnum_write_to!(LE<i64>, 8, |b: &mut [u8], n: Self| LittleEndian::write_i64(b,n.0));
