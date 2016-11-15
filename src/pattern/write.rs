use byteorder::ByteOrder;
use byteorder::BigEndian;
use byteorder::LittleEndian;

#[derive(Debug)]
pub struct U8(pub u8);
impl U8 {
    pub fn write(self, buf: &mut [u8]) {
        buf[0] = self.0;
    }
}

#[derive(Debug)]
pub struct U16le(pub u16);
impl U16le {
    pub fn write(self, buf: &mut [u8]) {
        LittleEndian::write_u16(buf, self.0);
    }
}

#[derive(Debug)]
pub struct U16be(pub u16);
impl U16be {
    pub fn write(self, buf: &mut [u8]) {
        BigEndian::write_u16(buf, self.0);
    }
}

#[derive(Debug)]
pub struct U24le(pub u32);
impl U24le {
    pub fn write(self, buf: &mut [u8]) {
        let mut tmp = [0; 4];
        LittleEndian::write_u32(&mut tmp, self.0);
        buf.copy_from_slice(&tmp[0..3]);
    }
}

#[derive(Debug)]
pub struct U24be(pub u32);
impl U24be {
    pub fn write(self, buf: &mut [u8]) {
        let mut tmp = [0; 4];
        BigEndian::write_u32(&mut tmp, self.0);
        buf.copy_from_slice(&tmp[1..4]);
    }
}

#[derive(Debug)]
pub struct U32le(pub u32);
impl U32le {
    pub fn write(self, buf: &mut [u8]) {
        LittleEndian::write_u32(buf, self.0);
    }
}

#[derive(Debug)]
pub struct U32be(pub u32);
impl U32be {
    pub fn write(self, buf: &mut [u8]) {
        BigEndian::write_u32(buf, self.0);
    }
}

#[derive(Debug)]
pub struct U64le(pub u64);
impl U64le {
    pub fn write(self, buf: &mut [u8]) {
        LittleEndian::write_u64(buf, self.0);
    }
}

#[derive(Debug)]
pub struct U64be(pub u64);
impl U64be {
    pub fn write(self, buf: &mut [u8]) {
        BigEndian::write_u64(buf, self.0);
    }
}

#[derive(Debug)]
pub struct I8(pub i8);
impl I8 {
    pub fn write(self, buf: &mut [u8]) {
        buf[0] = self.0 as u8;
    }
}

#[derive(Debug)]
pub struct I16le(pub i16);
impl I16le {
    pub fn write(self, buf: &mut [u8]) {
        LittleEndian::write_i16(buf, self.0);
    }
}

#[derive(Debug)]
pub struct I16be(pub i16);
impl I16be {
    pub fn write(self, buf: &mut [u8]) {
        BigEndian::write_i16(buf, self.0);
    }
}

#[derive(Debug)]
pub struct I24le(pub i32);
impl I24le {
    pub fn write(self, buf: &mut [u8]) {
        let mut tmp = [0; 4];
        LittleEndian::write_i32(&mut tmp, self.0);
        buf.copy_from_slice(&tmp[0..3]);
    }
}

#[derive(Debug)]
pub struct I24be(pub i32);
impl I24be {
    pub fn write(self, buf: &mut [u8]) {
        let mut tmp = [0; 4];
        BigEndian::write_i32(&mut tmp, self.0);
        buf.copy_from_slice(&tmp[1..4]);
    }
}

#[derive(Debug)]
pub struct I32le(pub i32);
impl I32le {
    pub fn write(self, buf: &mut [u8]) {
        LittleEndian::write_i32(buf, self.0);
    }
}

#[derive(Debug)]
pub struct I32be(pub i32);
impl I32be {
    pub fn write(self, buf: &mut [u8]) {
        BigEndian::write_i32(buf, self.0);
    }
}

#[derive(Debug)]
pub struct I64le(pub i64);
impl I64le {
    pub fn write(self, buf: &mut [u8]) {
        LittleEndian::write_i64(buf, self.0);
    }
}

#[derive(Debug)]
pub struct I64be(pub i64);
impl I64be {
    pub fn write(self, buf: &mut [u8]) {
        BigEndian::write_i64(buf, self.0);
    }
}
