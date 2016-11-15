use byteorder::ByteOrder;
use byteorder::BigEndian;
use byteorder::LittleEndian;

pub trait Fixed {
    type Output;
    fn convert(buf: &[u8]) -> Self::Output;
}

#[derive(Debug)]
pub struct U8;
impl Fixed for U8 {
    type Output = u8;
    fn convert(buf: &[u8]) -> Self::Output {
        buf[0]
    }
}

#[derive(Debug)]
pub struct U16le;
impl Fixed for U16le {
    type Output = u16;
    fn convert(buf: &[u8]) -> Self::Output {
        LittleEndian::read_u16(buf)
    }
}

#[derive(Debug)]
pub struct U16be;
impl Fixed for U16be {
    type Output = u16;
    fn convert(buf: &[u8]) -> Self::Output {
        BigEndian::read_u16(buf)
    }
}

#[derive(Debug)]
pub struct U24le;
impl Fixed for U24le {
    type Output = u32;
    fn convert(buf: &[u8]) -> Self::Output {
        LittleEndian::read_u32(&[buf[0], buf[1], buf[2], 0])
    }
}

#[derive(Debug)]
pub struct U24be;
impl Fixed for U24be {
    type Output = u32;
    fn convert(buf: &[u8]) -> Self::Output {
        BigEndian::read_u32(&[0, buf[0], buf[1], buf[2]])
    }
}

#[derive(Debug)]
pub struct U32le;
impl Fixed for U32le {
    type Output = u32;
    fn convert(buf: &[u8]) -> Self::Output {
        LittleEndian::read_u32(buf)
    }
}

#[derive(Debug)]
pub struct U32be;
impl Fixed for U32be {
    type Output = u32;
    fn convert(buf: &[u8]) -> Self::Output {
        BigEndian::read_u32(buf)
    }
}

#[derive(Debug)]
pub struct U64le;
impl Fixed for U64le {
    type Output = u64;
    fn convert(buf: &[u8]) -> Self::Output {
        LittleEndian::read_u64(buf)
    }
}

#[derive(Debug)]
pub struct U64be;
impl Fixed for U64be {
    type Output = u64;
    fn convert(buf: &[u8]) -> Self::Output {
        BigEndian::read_u64(buf)
    }
}

#[derive(Debug)]
pub struct I8;
impl Fixed for I8 {
    type Output = i8;
    fn convert(buf: &[u8]) -> Self::Output {
        buf[0] as i8
    }
}

#[derive(Debug)]
pub struct I16le;
impl Fixed for I16le {
    type Output = i16;
    fn convert(buf: &[u8]) -> Self::Output {
        LittleEndian::read_i16(buf)
    }
}

#[derive(Debug)]
pub struct I16be;
impl Fixed for I16be {
    type Output = i16;
    fn convert(buf: &[u8]) -> Self::Output {
        BigEndian::read_i16(buf)
    }
}

#[derive(Debug)]
pub struct I24le;
impl Fixed for I24le {
    type Output = i32;
    fn convert(buf: &[u8]) -> Self::Output {
        LittleEndian::read_i32(&[0, buf[0], buf[1], buf[2]]) >> 8
    }
}

#[derive(Debug)]
pub struct I24be;
impl Fixed for I24be {
    type Output = i32;
    fn convert(buf: &[u8]) -> Self::Output {
        BigEndian::read_i32(&[buf[0], buf[1], buf[2], 0]) >> 8
    }
}

#[derive(Debug)]
pub struct I32le;
impl Fixed for I32le {
    type Output = i32;
    fn convert(buf: &[u8]) -> Self::Output {
        LittleEndian::read_i32(buf)
    }
}

#[derive(Debug)]
pub struct I32be;
impl Fixed for I32be {
    type Output = i32;
    fn convert(buf: &[u8]) -> Self::Output {
        BigEndian::read_i32(buf)
    }
}

#[derive(Debug)]
pub struct I64le;
impl Fixed for I64le {
    type Output = i64;
    fn convert(buf: &[u8]) -> Self::Output {
        LittleEndian::read_i64(buf)
    }
}

#[derive(Debug)]
pub struct I64be;
impl Fixed for I64be {
    type Output = i64;
    fn convert(buf: &[u8]) -> Self::Output {
        BigEndian::read_i64(buf)
    }
}
