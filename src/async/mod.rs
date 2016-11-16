use std::io;
use futures::BoxFuture;

pub use self::read::AsyncRead;
pub use self::read::ReadPattern;
pub use self::read::Read;
pub use self::read::ReadExact;
pub use self::read::ReadToEnd;
pub use self::write::AsyncWrite;
pub use self::write::WritePattern;
pub use self::write::Write;
pub use self::write::WriteAll;

pub mod read;
mod write;

pub type IoFuture<T> = BoxFuture<T, io::Error>;

pub struct Window<B> {
    pub inner: B,
    pub offset: usize,
}
impl<B> Window<B> {
    pub fn new(inner: B) -> Self {
        Window::with_offset(inner, 0)
    }
    pub fn with_offset(inner: B, offset: usize) -> Self {
        Window {
            inner: inner,
            offset: offset,
        }
    }
}
impl<B> AsRef<[u8]> for Window<B>
    where B: AsRef<[u8]>
{
    fn as_ref(&self) -> &[u8] {
        &self.inner.as_ref()[self.offset..]
    }
}
impl<B> AsMut<[u8]> for Window<B>
    where B: AsMut<[u8]>
{
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[self.offset..]
    }
}
