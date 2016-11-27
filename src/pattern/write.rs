//! Patterns specific to writing operation.
use super::{Pattern, Endian};

/// A pattern which indicates to flush internal buffer.
#[derive(Debug, Clone)]
pub struct Flush;
impl Pattern for Flush {
    type Value = ();
}

impl Pattern for u8 {
    type Value = ();
}
impl Pattern for i8 {
    type Value = ();
}

impl Pattern for u16 {
    type Value = ();
}
impl Endian for u16 {}
impl Pattern for i16 {
    type Value = ();
}
impl Endian for i16 {}

impl Pattern for u32 {
    type Value = ();
}
impl Endian for u32 {}
impl Pattern for i32 {
    type Value = ();
}
impl Endian for i32 {}

impl Pattern for u64 {
    type Value = ();
}
impl Endian for u64 {}
impl Pattern for i64 {
    type Value = ();
}
impl Endian for i64 {}

/// A pattern associated to 24-bit unsigned integers.
#[derive(Debug, Clone)]
pub struct U24(pub u32);
impl Pattern for U24 {
    type Value = ();
}
impl Endian for U24 {}

/// A pattern associated to 24-bit signed integers.
#[derive(Debug, Clone)]
pub struct I24(pub i32);
impl Pattern for I24 {
    type Value = ();
}
impl Endian for I24 {}

/// A pattern associated to 40-bit unsigned integers.
#[derive(Debug, Clone)]
pub struct U40(pub u64);
impl Pattern for U40 {
    type Value = ();
}
impl Endian for U40 {}

/// A pattern associated to 40-bit signed integers.
#[derive(Debug, Clone)]
pub struct I40(pub i64);
impl Pattern for I40 {
    type Value = ();
}
impl Endian for I40 {}

/// A pattern associated to 48-bit unsigned integers.
#[derive(Debug, Clone)]
pub struct U48(pub u64);
impl Pattern for U48 {
    type Value = ();
}
impl Endian for U48 {}

/// A pattern associated to 48-bit signed integers.
#[derive(Debug, Clone)]
pub struct I48(pub i64);
impl Pattern for I48 {
    type Value = ();
}
impl Endian for I48 {}

/// A pattern associated to 56-bit unsigned integers.
#[derive(Debug, Clone)]
pub struct U56(pub u64);
impl Pattern for U56 {
    type Value = ();
}
impl Endian for U56 {}

/// A pattern associated to 56-bit signed integers.
#[derive(Debug, Clone)]
pub struct I56(pub i64);
impl Pattern for I56 {
    type Value = ();
}
impl Endian for I56 {}
