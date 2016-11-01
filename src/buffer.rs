pub trait BufferLike: Sized + AsRef<[u8]> + AsMut<[u8]> {
    type Value;
    fn into_value(self) -> Self::Value;
    fn from_value(value: Self::Value) -> Self;
}

pub struct U8([u8; 1]);
impl U8 {
    pub fn new() -> Self {
        U8([0])
    }
}
impl BufferLike for U8 {
    type Value = u8;
    fn into_value(self) -> Self::Value {
        self.0[0]
    }
    fn from_value(x: Self::Value) -> Self {
        U8([x])
    }
}
impl AsRef<[u8]> for U8 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl AsMut<[u8]> for U8 {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}
