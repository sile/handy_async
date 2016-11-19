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

impl<R: io::Read, P0, P1> ReadPattern<R> for (P0, P1)
    where P0: ReadPattern<R>,
          P1: ReadPattern<R>
{
    type Output = (P0::Output, P1::Output);
    fn sync_read_pattern(self, reader: &mut R) -> io::Result<Self::Output> {
        Ok((self.0.sync_read_pattern(reader)?, self.1.sync_read_pattern(reader)?))
    }
}
impl<R: io::Read, P0, P1, P2> ReadPattern<R> for (P0, P1, P2)
    where P0: ReadPattern<R>,
          P1: ReadPattern<R>,
          P2: ReadPattern<R>
{
    type Output = (P0::Output, P1::Output, P2::Output);
    fn sync_read_pattern(self, reader: &mut R) -> io::Result<Self::Output> {
        Ok((self.0.sync_read_pattern(reader)?,
            self.1.sync_read_pattern(reader)?,
            self.2.sync_read_pattern(reader)?))
    }
}
impl<R: io::Read, P0, P1, P2, P3> ReadPattern<R> for (P0, P1, P2, P3)
    where P0: ReadPattern<R>,
          P1: ReadPattern<R>,
          P2: ReadPattern<R>,
          P3: ReadPattern<R>
{
    type Output = (P0::Output, P1::Output, P2::Output, P3::Output);
    fn sync_read_pattern(self, reader: &mut R) -> io::Result<Self::Output> {
        Ok((self.0.sync_read_pattern(reader)?,
            self.1.sync_read_pattern(reader)?,
            self.2.sync_read_pattern(reader)?,
            self.3.sync_read_pattern(reader)?))
    }
}
impl<R: io::Read, P0, P1, P2, P3, P4> ReadPattern<R> for (P0, P1, P2, P3, P4)
    where P0: ReadPattern<R>,
          P1: ReadPattern<R>,
          P2: ReadPattern<R>,
          P3: ReadPattern<R>,
          P4: ReadPattern<R>
{
    type Output = (P0::Output, P1::Output, P2::Output, P3::Output, P4::Output);
    fn sync_read_pattern(self, reader: &mut R) -> io::Result<Self::Output> {
        Ok((self.0.sync_read_pattern(reader)?,
            self.1.sync_read_pattern(reader)?,
            self.2.sync_read_pattern(reader)?,
            self.3.sync_read_pattern(reader)?,
            self.4.sync_read_pattern(reader)?))
    }
}
