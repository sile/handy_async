//! I/O operation related components.
pub use self::async_read::AsyncRead;
pub use self::async_write::AsyncWrite;
pub use self::read_pattern::{ReadPattern, PatternReader};
pub use self::write_pattern::{WritePattern, PatternWriter};

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

use std::fmt;
use std::error;
use std::io::Error;

pub struct AsyncError<T> {
    state: T,
    error: Error,
}
impl<T> AsyncError<T> {
    pub fn new(state: T, error: Error) -> Self {
        AsyncError {
            state: state,
            error: error,
        }
    }
    pub fn map<F, U>(self, f: F) -> AsyncError<U>
        where F: FnOnce(T) -> U
    {
        AsyncError {
            state: f(self.state),
            error: self.error,
        }
    }
    pub fn state_ref(&self) -> &T {
        &self.state
    }
    pub fn state_muf(&mut self) -> &mut T {
        &mut self.state
    }
    pub fn error_ref(&self) -> &Error {
        &self.error
    }
    pub fn error_mut(&mut self) -> &mut Error {
        &mut self.error
    }
    pub fn into_error(self) -> Error {
        self.error
    }
    pub fn unwrap(self) -> (T, Error) {
        (self.state, self.error)
    }
}
impl<T> From<AsyncError<T>> for Error {
    fn from(f: AsyncError<T>) -> Self {
        f.into_error()
    }
}
impl<T> fmt::Debug for AsyncError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AsyncError {{ state: _, error: {:?} }}", self.error)
    }
}
impl<T> fmt::Display for AsyncError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Asynchronous I/O Error: {}", self.error)
    }
}
impl<T> error::Error for AsyncError<T> {
    fn description(&self) -> &str {
        self.error.description()
    }
    fn cause(&self) -> Option<&error::Error> {
        self.error.cause()
    }
}
