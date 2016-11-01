use buffer;
use super::Pattern;

pub struct U8;
impl Pattern for U8 {
    type Buffer = buffer::U8;
    fn into_buffer(self) -> Self::Buffer {
        buffer::U8::new()
    }
}
