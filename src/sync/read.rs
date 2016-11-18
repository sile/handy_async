use std::io;

use pattern;

pub trait ReadPattern<R: io::Read> {
    type Output;
    fn sync_read_pattern(self, reader: &mut R) -> io::Result<Self::Output>;
}

macro_rules! impl_fixed_read_pattern {
    ($p:ident, $b:expr) => {
        impl<R: io::Read> ReadPattern<R> for pattern::read::$p {
            type Output = <pattern::read::$p as pattern::read::Fixed>::Output;
            fn sync_read_pattern(self, reader: &mut R) -> io::Result<Self::Output> {
                use pattern::read::Fixed;
                let mut buf = [0; $b];
                reader.read_exact(&mut buf)?;
                Ok(Self::convert(&buf))
            }
        }
    }
}
impl_fixed_read_pattern!(U8, 1);
impl_fixed_read_pattern!(U16le, 2);
impl_fixed_read_pattern!(U16be, 2);
impl_fixed_read_pattern!(U24le, 3);
impl_fixed_read_pattern!(U24be, 3);
impl_fixed_read_pattern!(U32le, 4);
impl_fixed_read_pattern!(U32be, 4);
impl_fixed_read_pattern!(U64le, 8);
impl_fixed_read_pattern!(U64be, 8);
impl_fixed_read_pattern!(I8, 1);
impl_fixed_read_pattern!(I16le, 2);
impl_fixed_read_pattern!(I16be, 2);
impl_fixed_read_pattern!(I24le, 3);
impl_fixed_read_pattern!(I24be, 3);
impl_fixed_read_pattern!(I32le, 4);
impl_fixed_read_pattern!(I32be, 4);
impl_fixed_read_pattern!(I64le, 8);
impl_fixed_read_pattern!(I64be, 8);
