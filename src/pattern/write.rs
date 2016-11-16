use byteorder::ByteOrder;
use byteorder::BigEndian;
use byteorder::LittleEndian;

pub trait Fixed {
    fn write(self, buf: &mut [u8]);
}

impl Fixed for u8 {
    fn write(self, buf: &mut [u8]) {
        buf[0] = self;
    }
}
impl Fixed for i8 {
    fn write(self, buf: &mut [u8]) {
        buf[0] = self as u8;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct U24(pub u32);

#[derive(Debug, Clone, Copy)]
pub struct I24(pub i32);

#[derive(Debug, Clone, Copy)]
pub struct BE<T>(pub T);
impl Fixed for BE<u16> {
    fn write(self, buf: &mut [u8]) {
        BigEndian::write_u16(buf, self.0);
    }
}
impl Fixed for BE<U24> {
    fn write(self, buf: &mut [u8]) {
        let mut tmp = [0; 4];
        BigEndian::write_u32(&mut tmp, (self.0).0);
        buf.copy_from_slice(&tmp[1..4]);
    }
}
impl Fixed for BE<u32> {
    fn write(self, buf: &mut [u8]) {
        BigEndian::write_u32(buf, self.0);
    }
}
impl Fixed for BE<u64> {
    fn write(self, buf: &mut [u8]) {
        BigEndian::write_u64(buf, self.0);
    }
}
impl Fixed for BE<i16> {
    fn write(self, buf: &mut [u8]) {
        BigEndian::write_i16(buf, self.0);
    }
}
impl Fixed for BE<I24> {
    fn write(self, buf: &mut [u8]) {
        let mut tmp = [0; 4];
        BigEndian::write_i32(&mut tmp, (self.0).0);
        buf.copy_from_slice(&tmp[1..4]);
    }
}
impl Fixed for BE<i32> {
    fn write(self, buf: &mut [u8]) {
        BigEndian::write_i32(buf, self.0);
    }
}
impl Fixed for BE<i64> {
    fn write(self, buf: &mut [u8]) {
        BigEndian::write_i64(buf, self.0);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LE<T>(pub T);
impl Fixed for LE<u16> {
    fn write(self, buf: &mut [u8]) {
        LittleEndian::write_u16(buf, self.0);
    }
}
impl Fixed for LE<U24> {
    fn write(self, buf: &mut [u8]) {
        let mut tmp = [0; 4];
        LittleEndian::write_u32(&mut tmp, (self.0).0);
        buf.copy_from_slice(&tmp[0..3]);
    }
}
impl Fixed for LE<u32> {
    fn write(self, buf: &mut [u8]) {
        LittleEndian::write_u32(buf, self.0);
    }
}
impl Fixed for LE<u64> {
    fn write(self, buf: &mut [u8]) {
        LittleEndian::write_u64(buf, self.0);
    }
}
impl Fixed for LE<i16> {
    fn write(self, buf: &mut [u8]) {
        LittleEndian::write_i16(buf, self.0);
    }
}
impl Fixed for LE<I24> {
    fn write(self, buf: &mut [u8]) {
        let mut tmp = [0; 4];
        LittleEndian::write_i32(&mut tmp, (self.0).0);
        buf.copy_from_slice(&tmp[0..3]);
    }
}
impl Fixed for LE<i32> {
    fn write(self, buf: &mut [u8]) {
        LittleEndian::write_i32(buf, self.0);
    }
}
impl Fixed for LE<i64> {
    fn write(self, buf: &mut [u8]) {
        LittleEndian::write_i64(buf, self.0);
    }
}

#[derive(Debug)]
pub struct Iter<I>(pub I);
