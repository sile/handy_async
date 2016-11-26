use std::io;
use std::mem;

use pattern::{Pattern, Branch};

pub enum Phase<A, B> {
    A(A),
    B(B),
    Done,
}
impl<A, B> Phase<A, B> {
    pub fn take(&mut self) -> Self {
        mem::replace(self, Phase::Done)
    }
}

pub fn then_to_and_then<F, T, U>(result: io::Result<(T, F)>) -> Branch<U, io::Result<U::Value>>
    where F: FnOnce(T) -> U,
          U: Pattern
{
    result.map(|(v, and_then)| Branch::A(and_then(v))).unwrap_or_else(|e| Branch::B(Err(e)))
}

pub fn and_then_to_map<F, T, U>((t, f): (T, F)) -> io::Result<U>
    where F: FnOnce(T) -> U
{
    Ok(f(t))
}
