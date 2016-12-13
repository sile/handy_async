//! Error related components.
use std::fmt;
use std::error;

/// The error type for asynchronous operations.
///
/// This contains an actual error value and the state that caused the error.
///
/// # Examples
///
/// ```
/// # extern crate handy_async;
/// # extern crate futures;
/// use std::io::{empty, ErrorKind};
/// use handy_async::io::AsyncRead;
/// use handy_async::error::AsyncError;
/// use futures::Future;
///
/// # fn main() {
/// // Trying to read any bytes from `empty()` stream will result in an `UnexpectedEof` error.
/// let e: AsyncError<_, _> = empty().async_read_non_empty([0]).wait().err().unwrap();
/// assert_eq!(e.error_ref().kind(), ErrorKind::UnexpectedEof);
/// # }
/// ```
pub struct AsyncError<T, E> {
    state: T,
    error: E,
}
impl<T, E> AsyncError<T, E>
    where E: error::Error
{
    /// Makes a new error instance.
    pub fn new(state: T, error: E) -> Self {
        AsyncError {
            state: state,
            error: error,
        }
    }

    /// Gets the immutable reference of the state of this `AsyncError`.
    pub fn state_ref(&self) -> &T {
        &self.state
    }

    /// Gets the mutable reference of the state of this `AsyncError`.
    pub fn state_muf(&mut self) -> &mut T {
        &mut self.state
    }

    /// Converts this `AsyncError` into the underlying state `T`.
    pub fn into_state(self) -> T {
        self.state
    }

    /// Gets the immutable reference of the actual error of this `AsyncError`.
    pub fn error_ref(&self) -> &E {
        &self.error
    }

    /// Gets the mutable reference of the actual error of this `AsyncError`.
    pub fn error_mut(&mut self) -> &mut E {
        &mut self.error
    }

    /// Converts this `AsyncError` into the underlying error `E`.
    pub fn into_error(self) -> E {
        self.error
    }

    /// Unwraps this `AsyncError`, returing the underlying state and error as `(T, E)`.
    pub fn unwrap(self) -> (T, E) {
        (self.state, self.error)
    }

    /// Maps a `AsyncError<T, E>` to `AsyncError<U, E>` by
    /// applying a function `F` to the contained state.
    ///
    /// # Examples
    /// ```
    /// use std::io::{Error, ErrorKind};
    /// use handy_async::error::AsyncError;
    ///
    /// let error = AsyncError::new("dummy_state", Error::new(ErrorKind::Other, ""));
    /// assert_eq!(error.state_ref(), &"dummy_state");
    ///
    /// let error = error.map_state(|s| s.len());
    /// assert_eq!(error.state_ref(), &11);
    /// ```
    pub fn map_state<F, U>(self, f: F) -> AsyncError<U, E>
        where F: FnOnce(T) -> U
    {
        AsyncError {
            state: f(self.state),
            error: self.error,
        }
    }
}
impl<T, E> fmt::Debug for AsyncError<T, E>
    where E: error::Error
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AsyncError {{ state: _, error: {:?} }}", self.error)
    }
}
impl<T, E> fmt::Display for AsyncError<T, E>
    where E: error::Error
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Asynchronous Error: {}", self.error)
    }
}
impl<T, E> error::Error for AsyncError<T, E>
    where E: error::Error
{
    fn description(&self) -> &str {
        self.error.description()
    }
    fn cause(&self) -> Option<&error::Error> {
        self.error.cause()
    }
}
