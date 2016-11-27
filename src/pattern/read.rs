//! Patterns specific to reading operation.
use super::{Pattern, Endian};

/// A pattern associated to 8-bit unsigned integers.
#[derive(Debug, Clone)]
pub struct U8;
impl Pattern for U8 {
    type Value = u8;
}

/// A pattern associated to 16-bit unsigned integers.
#[derive(Debug, Clone)]
pub struct U16;
impl Pattern for U16 {
    type Value = u16;
}
impl Endian for U16 {}

/// A pattern associated to 24-bit unsigned integers.
#[derive(Debug, Clone)]
pub struct U24;
impl Pattern for U24 {
    type Value = u32;
}
impl Endian for U24 {}

/// A pattern associated to 32-bit unsigned integers.
#[derive(Debug, Clone)]
pub struct U32;
impl Pattern for U32 {
    type Value = u32;
}
impl Endian for U32 {}

/// A pattern associated to 40-bit unsigned integers.
#[derive(Debug, Clone)]
pub struct U40;
impl Pattern for U40 {
    type Value = u64;
}
impl Endian for U40 {}

/// A pattern associated to 48-bit unsigned integers.
#[derive(Debug, Clone)]
pub struct U48;
impl Pattern for U48 {
    type Value = u64;
}
impl Endian for U48 {}

/// A pattern associated to 56-bit unsigned integers.
#[derive(Debug, Clone)]
pub struct U56;
impl Pattern for U56 {
    type Value = u64;
}
impl Endian for U56 {}

/// A pattern associated to 64-bit unsigned integers.
#[derive(Debug, Clone)]
pub struct U64;
impl Pattern for U64 {
    type Value = u64;
}
impl Endian for U64 {}

/// A pattern associated to 8-bit signed integers.
#[derive(Debug, Clone)]
pub struct I8;
impl Pattern for I8 {
    type Value = i8;
}

/// A pattern associated to 16-bit signed integers.
#[derive(Debug, Clone)]
pub struct I16;
impl Pattern for I16 {
    type Value = i16;
}
impl Endian for I16 {}

/// A pattern associated to 24-bit signed integers.
#[derive(Debug, Clone)]
pub struct I24;
impl Pattern for I24 {
    type Value = i32;
}
impl Endian for I24 {}

/// A pattern associated to 32-bit signed integers.
#[derive(Debug, Clone)]
pub struct I32;
impl Pattern for I32 {
    type Value = i32;
}
impl Endian for I32 {}

/// A pattern associated to 40-bit signed integers.
#[derive(Debug, Clone)]
pub struct I40;
impl Pattern for I40 {
    type Value = i64;
}
impl Endian for I40 {}

/// A pattern associated to 48-bit signed integers.
#[derive(Debug, Clone)]
pub struct I48;
impl Pattern for I48 {
    type Value = i64;
}
impl Endian for I48 {}

/// A pattern associated to 56-bit signed integers.
#[derive(Debug, Clone)]
pub struct I56;
impl Pattern for I56 {
    type Value = i64;
}
impl Endian for I56 {}

/// A pattern associated to 64-bit signed integers.
#[derive(Debug, Clone)]
pub struct I64;
impl Pattern for I64 {
    type Value = i64;
}
impl Endian for I64 {}

/// A pattern associated to 32-bit floating point numbers.
#[derive(Debug, Clone)]
pub struct F32;
impl Pattern for F32 {
    type Value = f32;
}
impl Endian for F32 {}

/// A pattern associated to 64-bit floating point numbers.
#[derive(Debug, Clone)]
pub struct F64;
impl Pattern for F64 {
    type Value = f64;
}
impl Endian for F64 {}

/// A pattern which indicates the 'End-Of-Stream'.
#[derive(Debug, Clone)]
pub struct Eos;
impl Pattern for Eos {
    type Value = Result<(), u8>;
}
