//! Synchronous I/O functionalities.

use std::io::{Result, Read, Write, Error, ErrorKind};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};

/// An extention of the standard `Read` trait.
pub trait ReadExt: Read {
    /// Reads a 8-bit unsigned integer.
    fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    /// Reads a big-endian 16-bit unsigned integer.
    fn read_u16be(&mut self) -> Result<u16> {
        self.read_u16::<BigEndian>()
    }

    /// Reads a little-endian 16-bit unsigned integer.
    fn read_u16le(&mut self) -> Result<u16> {
        self.read_u16::<LittleEndian>()
    }

    /// Reads a big-endian 24-bit unsigned integer.
    fn read_u24be(&mut self) -> Result<u32> {
        self.read_uint::<BigEndian>(3).map(|n| n as u32)
    }

    /// Reads a little-endian 24-bit unsigned integer.
    fn read_u24le(&mut self) -> Result<u32> {
        self.read_uint::<LittleEndian>(3).map(|n| n as u32)
    }

    /// Reads a big-endian 32-bit unsigned integer.
    fn read_u32be(&mut self) -> Result<u32> {
        self.read_u32::<BigEndian>()
    }

    /// Reads a little-endian 32-bit unsigned integer.
    fn read_u32le(&mut self) -> Result<u32> {
        self.read_u32::<LittleEndian>()
    }

    /// Reads a big-endian 40-bit unsigned integer.
    fn read_u40be(&mut self) -> Result<u64> {
        self.read_uint::<BigEndian>(5)
    }

    /// Reads a little-endian 40-bit unsigned integer.
    fn read_u40le(&mut self) -> Result<u64> {
        self.read_uint::<LittleEndian>(5)
    }

    /// Reads a big-endian 48-bit unsigned integer.
    fn read_u48be(&mut self) -> Result<u64> {
        self.read_uint::<BigEndian>(6)
    }

    /// Reads a little-endian 48-bit unsigned integer.
    fn read_u48le(&mut self) -> Result<u64> {
        self.read_uint::<LittleEndian>(6)
    }

    /// Reads a big-endian 56-bit unsigned integer.
    fn read_u56be(&mut self) -> Result<u64> {
        self.read_uint::<BigEndian>(7)
    }

    /// Reads a little-endian 56-bit unsigned integer.
    fn read_u56le(&mut self) -> Result<u64> {
        self.read_uint::<LittleEndian>(7)
    }

    /// Reads a big-endian 64-bit unsigned integer.
    fn read_u64be(&mut self) -> Result<u64> {
        self.read_u64::<BigEndian>()
    }

    /// Reads a little-endian 64-bit unsigned integer.
    fn read_u64le(&mut self) -> Result<u64> {
        self.read_u64::<LittleEndian>()
    }

    /// Reads string.
    fn read_string(&mut self, length: usize) -> Result<String> {
        let bytes = self.read_bytes(length)?;
        let string = String::from_utf8(bytes).map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })?;
        Ok(string)
    }

    /// Reads bytes.
    fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0; length];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Reads all string.
    fn read_all_string(&mut self) -> Result<String> {
        let mut buf = String::new();
        self.read_to_string(&mut buf)?;
        Ok(buf)
    }

    /// Reads all bytes.
    fn read_all_bytes(&mut self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.read_to_end(&mut buf)?;
        Ok(buf)
    }
}
impl<R: Read> ReadExt for R {}

/// An extention of the standard `Write` trait.
pub trait WriteExt: Write {
    /// Writes a 8-bit unsigned integer.
    fn write_u8(&mut self, n: u8) -> Result<()> {
        self.write_all(&[n][..])?;
        Ok(())
    }

    /// Writes a big-endian 16-bit integer.
    fn write_u16be(&mut self, n: u16) -> Result<()> {
        self.write_u16::<BigEndian>(n)
    }

    /// Writes a little-endian 16-bit integer.
    fn write_u16le(&mut self, n: u16) -> Result<()> {
        self.write_u16::<LittleEndian>(n)
    }

    /// Writes a big-endian 24-bit integer.
    fn write_u24be(&mut self, n: u32) -> Result<()> {
        self.write_uint::<BigEndian>(u64::from(n), 3)
    }

    /// Writes a little-endian 24-bit integer.
    fn write_u24le(&mut self, n: u32) -> Result<()> {
        self.write_uint::<LittleEndian>(u64::from(n), 3)
    }

    /// Writes a big-endian 32-bit integer.
    fn write_u32be(&mut self, n: u32) -> Result<()> {
        self.write_u32::<BigEndian>(n)
    }

    /// Writes a little-endian 32-bit integer.
    fn write_u32le(&mut self, n: u32) -> Result<()> {
        self.write_u32::<LittleEndian>(n)
    }

    /// Writes a big-endian 40-bit integer.
    fn write_u40be(&mut self, n: u64) -> Result<()> {
        self.write_uint::<BigEndian>(n, 5)
    }

    /// Writes a little-endian 40-bit integer.
    fn write_u40le(&mut self, n: u64) -> Result<()> {
        self.write_uint::<LittleEndian>(n, 5)
    }

    /// Writes a big-endian 48-bit integer.
    fn write_u48be(&mut self, n: u64) -> Result<()> {
        self.write_uint::<BigEndian>(n, 6)
    }

    /// Writes a little-endian 48-bit integer.
    fn write_u48le(&mut self, n: u64) -> Result<()> {
        self.write_uint::<LittleEndian>(n, 6)
    }

    /// Writes a big-endian 56-bit integer.
    fn write_u56be(&mut self, n: u64) -> Result<()> {
        self.write_uint::<BigEndian>(n, 7)
    }

    /// Writes a little-endian 56-bit integer.
    fn write_u56le(&mut self, n: u64) -> Result<()> {
        self.write_uint::<LittleEndian>(n, 7)
    }

    /// Writes a big-endian 64-bit integer.
    fn write_u64be(&mut self, n: u64) -> Result<()> {
        self.write_u64::<BigEndian>(n)
    }

    /// Writes a little-endian 64-bit integer.
    fn write_u64le(&mut self, n: u64) -> Result<()> {
        self.write_u64::<LittleEndian>(n)
    }
}
impl<W: Write> WriteExt for W {}
