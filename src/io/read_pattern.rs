use std;
use std::io::{Read, Error, ErrorKind, Result};
use futures::{Poll, Async, Future};
use byteorder::{ByteOrder, NativeEndian, BigEndian, LittleEndian};

use io::AsyncRead;
use io::futures::{ReadBytes, ReadExact, ReadNonEmpty};
use pattern::{Pattern, AsyncMatch, Buf, Window};
use pattern::read;
use pattern::combinators::{self, BE, LE, PartialBuf};

pub struct PatternReader<R>(R);
impl<R: Read> Read for PatternReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.0.read(buf)
    }
}

pub trait ReadPattern<R: Read>: AsyncMatch<PatternReader<R>, Error> {
    fn read_from(self, reader: R) -> ReadFrom<Self, R> {
        ReadFrom(self.async_match(PatternReader(reader)))
    }
}
impl<R: Read, T> ReadPattern<R> for T where T: AsyncMatch<PatternReader<R>, Error> {}

pub struct ReadFrom<P, R>(P::Future) where P: AsyncMatch<PatternReader<R>, Error>;
impl<P, R> ReadFrom<P, R>
    where P: AsyncMatch<PatternReader<R>, Error>
{
    pub fn lossy(self) -> LossyReadFrom<P, R> {
        LossyReadFrom(self)
    }
    pub fn await(self) -> Result<P::Value> {
        self.wait().map(|(_, v)| v).map_err(|(_, e)| e)
    }
}
impl<P, R> Future for ReadFrom<P, R>
    where P: AsyncMatch<PatternReader<R>, Error>
{
    type Item = (R, P::Value);
    type Error = (R, Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(self.0.poll().map_err(|(m, v)| (m.0, v))?.map(|(m, v)| (m.0, v)))
    }
}

pub struct LossyReadFrom<P, R>(ReadFrom<P, R>) where P: AsyncMatch<PatternReader<R>, Error>;
impl<P, R> Future for LossyReadFrom<P, R>
    where P: AsyncMatch<PatternReader<R>, Error>
{
    type Item = (R, P::Value);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(self.0.poll().map_err(|(_, e)| e)?)
    }
}

pub struct ReadBuf<R, B>(ReadExact<PatternReader<R>, Buf<B>>);
impl<R: Read, B: AsMut<[u8]>> Future for ReadBuf<R, B> {
    type Item = (PatternReader<R>, B);
    type Error = (PatternReader<R>, Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(self.0.poll().map_err(|(r, _, e)| (r, e))?.map(|(r, v)| (r, v.0)))
    }
}
impl<R: Read, B: AsMut<[u8]>> AsyncMatch<PatternReader<R>, Error> for Buf<B> {
    type Future = ReadBuf<R, B>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        ReadBuf(matcher.async_read_exact(self))
    }
}
impl<R: Read> AsyncMatch<PatternReader<R>, Error> for Vec<u8> {
    type Future = ReadBuf<R, Self>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        Buf(self).async_match(matcher)
    }
}
impl<R: Read, B: AsMut<[u8]>> AsyncMatch<PatternReader<R>, Error> for Window<B> {
    type Future = ReadBuf<R, Self>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        Buf(self).async_match(matcher)
    }
}

pub struct ReadPartialBuf<R, B>(ReadNonEmpty<PatternReader<R>, B>);
impl<R: Read, B: AsMut<[u8]>> Future for ReadPartialBuf<R, B> {
    type Item = (PatternReader<R>, (B, usize));
    type Error = (PatternReader<R>, Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll().map(|x| x.map(|(r, b, s)| (r, (b, s)))).map_err(|(r, _, e)| (r, e))
    }
}
impl<R: Read, B: AsMut<[u8]>> AsyncMatch<PatternReader<R>, Error> for PartialBuf<B> {
    type Future = ReadPartialBuf<R, B>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        ReadPartialBuf(matcher.async_read_non_empty(self.0))
    }
}

pub struct ReadString<R>(ReadExact<PatternReader<R>, Vec<u8>>);
impl<R: Read> Future for ReadString<R> {
    type Item = (PatternReader<R>, String);
    type Error = (PatternReader<R>, Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((r, b)) = self.0.poll().map_err(|(r, _, e)| (r, e))? {
            match String::from_utf8(b) {
                Ok(s) => Ok(Async::Ready((r, s))),
                Err(e) => Err((r, Error::new(ErrorKind::InvalidData, Box::new(e)))),
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}
impl<R: Read> AsyncMatch<PatternReader<R>, Error> for String {
    type Future = ReadString<R>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        ReadString(matcher.async_read_exact(self.into_bytes()))
    }
}

pub type ReadFixnum<R, P, T> where P: Pattern =
    <combinators::Map<P, fn(P::Value) -> T> as AsyncMatch<PatternReader<R>, Error>>::Future;
macro_rules! impl_read_fixnum_pattern {
    ($pat:ty, $val:ident, $size:expr, $conv:expr) => {
        impl<R: Read> AsyncMatch<PatternReader<R>, Error> for $pat {
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

pub struct ReadEos<R>(ReadExact<PatternReader<R>, [u8; 1]>);
impl<R: Read> Future for ReadEos<R> {
    type Item = (PatternReader<R>, std::result::Result<(), u8>);
    type Error = (PatternReader<R>, Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.poll() {
            Err((r, _, e)) => {
                if e.kind() == ErrorKind::UnexpectedEof {
                    Ok(Async::Ready((r, Ok(()))))
                } else {
                    Err((r, e))
                }
            }
            Ok(Async::Ready((r, b))) => Ok(Async::Ready((r, Err(b[0])))),
            Ok(Async::NotReady) => Ok(Async::NotReady),
        }
    }
}
impl<R: Read> AsyncMatch<PatternReader<R>, Error> for read::Eos {
    type Future = ReadEos<R>;
    fn async_match(self, matcher: PatternReader<R>) -> Self::Future {
        ReadEos(matcher.async_read_exact([0; 1]))
    }
}

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
    type Error = (PatternReader<R>, Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((r, mut b, read_size)) = self.read
            .poll()
            .map_err(|(r, _, e)| (r, e))? {
            let is_eos = read_size == 0;
            b = b.skip(read_size);
            let total_read_size = b.start();
            match (self.pred)(&b.inner_ref()[0..total_read_size], is_eos) {
                Err(e) => Err((r, e)),
                Ok(Some(v)) => {
                    let mut b = b.into_inner();
                    b.truncate(total_read_size);
                    Ok(Async::Ready((r, (b, v))))
                }
                Ok(None) if is_eos => {
                    Err((r, Error::new(ErrorKind::UnexpectedEof, "Unexpected Eof")))
                }
                Ok(None) => {
                    if b.as_ref().is_empty() {
                        use std::cmp;
                        let new_len = cmp::min(total_read_size * 2, self.max_buffer_size);
                        let mut inner = b.into_inner();
                        if new_len == inner.len() {
                            let message = format!("Buffer size limit ({} bytes) reached",
                                                  self.max_buffer_size);
                            return Err((r, Error::new(ErrorKind::Other, message)));
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
impl<R: Read, F, T> AsyncMatch<PatternReader<R>, Error> for read::Until<F, T>
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
