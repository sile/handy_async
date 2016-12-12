//! I/O operation related components.
use std::io;

pub use self::async_read::AsyncRead;
pub use self::async_write::AsyncWrite;
pub use self::read_pattern::{ReadPattern, PatternReader};
pub use self::write_pattern::{WritePattern, PatternWriter};

use error::AsyncError;

pub mod futures {
    pub use super::async_read::{ReadBytes, ReadNonEmpty, ReadExact};
    pub use super::read_pattern::{ReadEos, ReadUntil};

    pub use super::async_write::{Flush, WriteBytes, WriteAll};
    pub use super::write_pattern::WriteBuf;
}

mod async_read;
mod async_write;
mod read_pattern;
mod write_pattern;

pub type AsyncIoError<T> = AsyncError<T, io::Error>;
