use buffer::BufferLike;

pub mod read;

pub trait Pattern: Sized {
    type Buffer: BufferLike;
    fn into_buffer(self) -> Self::Buffer;
}
