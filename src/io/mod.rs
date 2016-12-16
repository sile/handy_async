//! I/O operation related components.
use std::io;

pub use self::async_read::AsyncRead;
pub use self::async_write::AsyncWrite;
pub use self::read_pattern::{ReadFrom, PatternReader};
pub use self::write_pattern::{WriteInto, PatternWriter};
pub use self::external_size::ExternalSize;

use error::AsyncError;

pub mod futures {
    //! I/O operation related futures.
    pub use super::async_read::{ReadBytes, ReadNonEmpty, ReadExact};
    pub use super::read_pattern::{ReadEos, ReadUntil, ReadBuf, ReadPartialBuf};
    pub use super::read_pattern::{ReadString, ReadFixnum, ReadPattern};
    pub use super::read_pattern::{ReadLengthPrefixedBytes, ReadLengthPrefixedStr};

    pub use super::async_write::{Flush, WriteBytes, WriteAll};
    pub use super::write_pattern::{WritePattern, WriteBuf, WritePartialBuf};
    pub use super::write_pattern::{WriteFixnum, WriteFlush};
}
pub mod streams {
    //! I/O operation related streams.
    pub use super::read_pattern::ReadStream;
}

pub mod misc;

mod async_read;
mod async_write;
mod read_pattern;
mod write_pattern;
mod external_size;

/// I/O specific asynchronous error type.
pub type AsyncIoError<T> = AsyncError<T, io::Error>;
