//! I/O operation related components.
use std::io;
use std::fmt;

pub use self::async_read::AsyncRead;
pub use self::async_write::AsyncWrite;
pub use self::read_pattern::{ReadFrom, PatternReader};
pub use self::write_pattern::{WriteInto, PatternWriter};
pub use self::external_size::ExternalSize;

use error::AsyncError;
use pattern::combinators::UnexpectedValue;

pub mod futures {
    //! I/O operation related futures.
    pub use super::async_read::{ReadBytes, ReadNonEmpty, ReadExact};
    pub use super::read_pattern::{ReadEos, ReadUntil, ReadBuf, ReadPartialBuf};
    pub use super::read_pattern::{ReadString, ReadFixnum, ReadPattern};
    pub use super::read_pattern::{ReadLengthPrefixedBytes, ReadUtf8, ReadAll};

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

impl<T> From<UnexpectedValue<T>> for io::Error
    where T: fmt::Debug
{
    fn from(f: UnexpectedValue<T>) -> Self {
        io::Error::new(io::ErrorKind::InvalidData,
                       format!("Unexpected value: {:?}", f.0))
    }
}


/// Stateful I/O stream.
#[derive(Debug, Clone)]
pub struct Stateful<S, T> {
    /// I/O stream.
    pub stream: S,

    /// State.
    pub state: T,
}
impl<S, T> Stateful<S, T> {
    /// Maps a `Stateful<S, T>` to `Stateful<T, U>` by
    /// applying a function `F` to the contained state.
    pub fn map_state<F, U>(self, f: F) -> Stateful<S, U>
        where F: FnOnce(T) -> U
    {
        let u = f(self.state);
        Stateful {
            stream: self.stream,
            state: u,
        }
    }
}
impl<S: io::Read, T> io::Read for Stateful<S, T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}
impl<S: io::Write, T> io::Write for Stateful<S, T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}
