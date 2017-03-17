use std::io::Result;

use pattern;
use pattern::combinators;
use pattern::read;
use pattern::write;

/// The `ExternalSize` trait allows for calculating external byte size issued
/// when an I/O operation is performed on a pattern.
///
/// # Examples
/// ```
/// use handy_async::io::ExternalSize;
/// use handy_async::pattern::read::{U8, U32};
///
/// let pattern = (U8, U32, "Hello World!".to_string());
/// assert_eq!(pattern.external_size(), 17);
/// ```
pub trait ExternalSize {
    /// Calculates external byte size issued when
    /// an I/O operation is performed on this.
    fn external_size(&self) -> usize;
}

impl ExternalSize for Vec<u8> {
    fn external_size(&self) -> usize {
        self.len()
    }
}
impl ExternalSize for String {
    fn external_size(&self) -> usize {
        self.len()
    }
}
impl<T: ExternalSize> ExternalSize for Option<T> {
    fn external_size(&self) -> usize {
        self.as_ref().map_or(0, |t| t.external_size())
    }
}
impl<T: ExternalSize> ExternalSize for Result<T> {
    fn external_size(&self) -> usize {
        self.as_ref().map(|t| t.external_size()).unwrap_or(0)
    }
}
impl<T> ExternalSize for pattern::Iter<T>
    where T: Iterator + Clone,
          T::Item: ExternalSize
{
    fn external_size(&self) -> usize {
        self.0
            .clone()
            .map(|t| t.external_size())
            .sum()
    }
}
impl<I, F, T> ExternalSize for combinators::IterFold<I, F, T>
    where I: Iterator + Clone,
          I::Item: ExternalSize
{
    fn external_size(&self) -> usize {
        self.iter_ref()
            .clone()
            .map(|t| t.external_size())
            .sum()
    }
}
impl<T> ExternalSize for pattern::Window<T> {
    fn external_size(&self) -> usize {
        self.end() - self.start()
    }
}
impl<T: AsRef<[u8]>> ExternalSize for pattern::Buf<T> {
    fn external_size(&self) -> usize {
        self.0.as_ref().len()
    }
}
impl<T: ExternalSize> ExternalSize for combinators::BE<T> {
    fn external_size(&self) -> usize {
        self.0.external_size()
    }
}
impl<T: ExternalSize> ExternalSize for combinators::LE<T> {
    fn external_size(&self) -> usize {
        self.0.external_size()
    }
}
impl<A, B, C, D, E, F, G, H> ExternalSize for pattern::Branch<A, B, C, D, E, F, G, H>
    where A: ExternalSize,
          B: ExternalSize,
          C: ExternalSize,
          D: ExternalSize,
          E: ExternalSize,
          F: ExternalSize,
          G: ExternalSize,
          H: ExternalSize
{
    fn external_size(&self) -> usize {
        match *self {
            pattern::Branch::A(ref x) => x.external_size(),
            pattern::Branch::B(ref x) => x.external_size(),
            pattern::Branch::C(ref x) => x.external_size(),
            pattern::Branch::D(ref x) => x.external_size(),
            pattern::Branch::E(ref x) => x.external_size(),
            pattern::Branch::F(ref x) => x.external_size(),
            pattern::Branch::G(ref x) => x.external_size(),
            pattern::Branch::H(ref x) => x.external_size(),
        }
    }
}
impl<T0, T1> ExternalSize for combinators::Chain<T0, T1>
    where T0: ExternalSize,
          T1: ExternalSize
{
    fn external_size(&self) -> usize {
        let t = self.inner_ref();
        t.0.external_size() + t.1.external_size()
    }
}
impl ExternalSize for () {
    fn external_size(&self) -> usize {
        0
    }
}
impl<T0, T1> ExternalSize for (T0, T1)
    where T0: ExternalSize,
          T1: ExternalSize
{
    fn external_size(&self) -> usize {
        self.0.external_size() + self.1.external_size()
    }
}
impl<T0, T1, T2> ExternalSize for (T0, T1, T2)
    where T0: ExternalSize,
          T1: ExternalSize,
          T2: ExternalSize
{
    fn external_size(&self) -> usize {
        self.0.external_size() + self.1.external_size() + self.2.external_size()
    }
}
impl<T0, T1, T2, T3> ExternalSize for (T0, T1, T2, T3)
    where T0: ExternalSize,
          T1: ExternalSize,
          T2: ExternalSize,
          T3: ExternalSize
{
    fn external_size(&self) -> usize {
        self.0.external_size() + self.1.external_size() + self.2.external_size() +
        self.3.external_size()
    }
}
impl<T0, T1, T2, T3, T4> ExternalSize for (T0, T1, T2, T3, T4)
    where T0: ExternalSize,
          T1: ExternalSize,
          T2: ExternalSize,
          T3: ExternalSize,
          T4: ExternalSize
{
    fn external_size(&self) -> usize {
        self.0.external_size() + self.1.external_size() + self.2.external_size() +
        self.3.external_size() + self.4.external_size()
    }
}
impl<T0, T1, T2, T3, T4, T5> ExternalSize for (T0, T1, T2, T3, T4, T5)
    where T0: ExternalSize,
          T1: ExternalSize,
          T2: ExternalSize,
          T3: ExternalSize,
          T4: ExternalSize,
          T5: ExternalSize
{
    fn external_size(&self) -> usize {
        self.0.external_size() + self.1.external_size() + self.2.external_size() +
        self.3.external_size() + self.4.external_size() + self.5.external_size()
    }
}
impl<T0, T1, T2, T3, T4, T5, T6> ExternalSize for (T0, T1, T2, T3, T4, T5, T6)
    where T0: ExternalSize,
          T1: ExternalSize,
          T2: ExternalSize,
          T3: ExternalSize,
          T4: ExternalSize,
          T5: ExternalSize,
          T6: ExternalSize
{
    fn external_size(&self) -> usize {
        self.0.external_size() + self.1.external_size() + self.2.external_size() +
        self.3.external_size() + self.4.external_size() + self.5.external_size() +
        self.6.external_size()
    }
}
impl<T0, T1, T2, T3, T4, T5, T6, T7> ExternalSize for (T0, T1, T2, T3, T4, T5, T6, T7)
    where T0: ExternalSize,
          T1: ExternalSize,
          T2: ExternalSize,
          T3: ExternalSize,
          T4: ExternalSize,
          T5: ExternalSize,
          T6: ExternalSize,
          T7: ExternalSize
{
    fn external_size(&self) -> usize {
        self.0.external_size() + self.1.external_size() + self.2.external_size() +
        self.3.external_size() + self.4.external_size() + self.5.external_size() +
        self.6.external_size() + self.7.external_size()
    }
}
impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> ExternalSize for (T0, T1, T2, T3, T4, T5, T6, T7, T8)
    where T0: ExternalSize,
          T1: ExternalSize,
          T2: ExternalSize,
          T3: ExternalSize,
          T4: ExternalSize,
          T5: ExternalSize,
          T6: ExternalSize,
          T7: ExternalSize,
          T8: ExternalSize
{
    fn external_size(&self) -> usize {
        self.0.external_size() + self.1.external_size() + self.2.external_size() +
        self.3.external_size() + self.4.external_size() + self.5.external_size() +
        self.6.external_size() + self.7.external_size() + self.8.external_size()
    }
}
impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> ExternalSize
    for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)
    where T0: ExternalSize,
          T1: ExternalSize,
          T2: ExternalSize,
          T3: ExternalSize,
          T4: ExternalSize,
          T5: ExternalSize,
          T6: ExternalSize,
          T7: ExternalSize,
          T8: ExternalSize,
          T9: ExternalSize
{
    fn external_size(&self) -> usize {
        self.0.external_size() + self.1.external_size() + self.2.external_size() +
        self.3.external_size() + self.4.external_size() + self.5.external_size() +
        self.6.external_size() +
        self.7.external_size() + self.8.external_size() + self.9.external_size()
    }
}
impl ExternalSize for write::Flush {
    fn external_size(&self) -> usize {
        0
    }
}
impl ExternalSize for u8 {
    fn external_size(&self) -> usize {
        1
    }
}
impl ExternalSize for i8 {
    fn external_size(&self) -> usize {
        1
    }
}
impl ExternalSize for u16 {
    fn external_size(&self) -> usize {
        2
    }
}
impl ExternalSize for i16 {
    fn external_size(&self) -> usize {
        2
    }
}
impl ExternalSize for write::U24 {
    fn external_size(&self) -> usize {
        3
    }
}
impl ExternalSize for write::I24 {
    fn external_size(&self) -> usize {
        3
    }
}
impl ExternalSize for u32 {
    fn external_size(&self) -> usize {
        4
    }
}
impl ExternalSize for i32 {
    fn external_size(&self) -> usize {
        4
    }
}
impl ExternalSize for write::U40 {
    fn external_size(&self) -> usize {
        5
    }
}
impl ExternalSize for write::I40 {
    fn external_size(&self) -> usize {
        5
    }
}
impl ExternalSize for write::U48 {
    fn external_size(&self) -> usize {
        6
    }
}
impl ExternalSize for write::I48 {
    fn external_size(&self) -> usize {
        6
    }
}
impl ExternalSize for write::U56 {
    fn external_size(&self) -> usize {
        7
    }
}
impl ExternalSize for write::I56 {
    fn external_size(&self) -> usize {
        7
    }
}
impl ExternalSize for u64 {
    fn external_size(&self) -> usize {
        8
    }
}
impl ExternalSize for i64 {
    fn external_size(&self) -> usize {
        8
    }
}
impl ExternalSize for read::U8 {
    fn external_size(&self) -> usize {
        1
    }
}
impl ExternalSize for read::I8 {
    fn external_size(&self) -> usize {
        1
    }
}
impl ExternalSize for read::U16 {
    fn external_size(&self) -> usize {
        2
    }
}
impl ExternalSize for read::I16 {
    fn external_size(&self) -> usize {
        2
    }
}
impl ExternalSize for read::U24 {
    fn external_size(&self) -> usize {
        3
    }
}
impl ExternalSize for read::I24 {
    fn external_size(&self) -> usize {
        3
    }
}
impl ExternalSize for read::U32 {
    fn external_size(&self) -> usize {
        4
    }
}
impl ExternalSize for read::I32 {
    fn external_size(&self) -> usize {
        4
    }
}
impl ExternalSize for read::U40 {
    fn external_size(&self) -> usize {
        5
    }
}
impl ExternalSize for read::I40 {
    fn external_size(&self) -> usize {
        5
    }
}
impl ExternalSize for read::U48 {
    fn external_size(&self) -> usize {
        6
    }
}
impl ExternalSize for read::I48 {
    fn external_size(&self) -> usize {
        6
    }
}
impl ExternalSize for read::U56 {
    fn external_size(&self) -> usize {
        7
    }
}
impl ExternalSize for read::I56 {
    fn external_size(&self) -> usize {
        7
    }
}
impl ExternalSize for read::U64 {
    fn external_size(&self) -> usize {
        8
    }
}
impl ExternalSize for read::I64 {
    fn external_size(&self) -> usize {
        8
    }
}
