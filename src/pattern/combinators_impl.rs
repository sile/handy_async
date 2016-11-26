use std::io;
use futures::{self, Poll, Async};

use super::{Pattern, Endian};

#[derive(Debug)]
pub struct Then<P, F>(P, F);
impl<P, F> Then<P, F> {
    pub fn unwrap(self) -> (P, F) {
        (self.0, self.1)
    }
}
impl<P0, P1, F> Pattern for Then<P0, F>
    where P0: Pattern,
          P1: Pattern,
          F: FnOnce(io::Result<P0::Value>) -> P1
{
    type Value = P1::Value;
}
pub fn then<P, F>(pattern: P, then: F) -> Then<P, F> {
    Then(pattern, then)
}

#[derive(Debug)]
pub struct AndThen<P, F>(P, F);
impl<P, F> AndThen<P, F> {
    pub fn unwrap(self) -> (P, F) {
        (self.0, self.1)
    }
}
impl<P0, P1, F> Pattern for AndThen<P0, F>
    where P0: Pattern,
          P1: Pattern,
          F: FnOnce(P0::Value) -> P1
{
    type Value = P1::Value;
}
pub fn and_then<P, F>(pattern: P, and_then: F) -> AndThen<P, F> {
    AndThen(pattern, and_then)
}

#[derive(Debug)]
pub struct OrElse<P, F>(P, F);
impl<P, F> OrElse<P, F> {
    pub fn unwrap(self) -> (P, F) {
        (self.0, self.1)
    }
}
impl<P0, P1, F> Pattern for OrElse<P0, F>
    where P0: Pattern,
          P1: Pattern<Value = P0::Value>,
          F: FnOnce(io::Error) -> P1
{
    type Value = P1::Value;
}
pub fn or_else<P, F>(pattern: P, or_else: F) -> OrElse<P, F> {
    OrElse(pattern, or_else)
}

#[derive(Debug)]
pub struct Map<P, F>(P, F);
impl<P, F> Map<P, F> {
    pub fn unwrap(self) -> (P, F) {
        (self.0, self.1)
    }
}
impl<P, F, T> Pattern for Map<P, F>
    where P: Pattern,
          F: FnOnce(P::Value) -> T
{
    type Value = T;
}
pub fn map<P, F>(pattern: P, map: F) -> Map<P, F> {
    Map(pattern, map)
}

#[derive(Debug, Clone)]
pub struct Chain<P0, P1>(P0, P1);
impl<P0, P1> Chain<P0, P1> {
    pub fn unwrap(self) -> (P0, P1) {
        (self.0, self.1)
    }
}
impl<P0, P1> Pattern for Chain<P0, P1>
    where P0: Pattern,
          P1: Pattern
{
    type Value = (P0::Value, P1::Value);
}
pub fn chain<P0, P1>(p0: P0, p1: P1) -> Chain<P0, P1> {
    Chain(p0, p1)
}

#[derive(Debug)]
pub struct IterFold<I, F, T>(I, F, T);
impl<I, F, T> IterFold<I, F, T> {
    pub fn unwrap(self) -> (I, F, T) {
        (self.0, self.1, self.2)
    }
}
impl<I, P, F, T> Pattern for IterFold<I, F, T>
    where I: Iterator<Item = P>,
          P: Pattern,
          F: Fn(T, P::Value) -> T
{
    type Value = T;
}
pub fn iter_fold<I, F, T>(iter: I, fold: F, init: T) -> IterFold<I, F, T> {
    IterFold(iter, fold, init)
}


#[derive(Debug, Clone)]
pub struct LE<T>(pub T);
impl<T> Pattern for LE<T>
    where T: Endian + Pattern
{
    type Value = T::Value;
}

#[derive(Debug, Clone)]
pub struct BE<T>(pub T);
impl<T> Pattern for BE<T>
    where T: Endian + Pattern
{
    type Value = T::Value;
}

#[derive(Debug, Clone)]
pub struct PartialBuf<B>(pub B);
impl<B: AsRef<[u8]>> AsRef<[u8]> for PartialBuf<B> {
    fn as_ref(&self) -> &[u8] {
        &self.0.as_ref()[..]
    }
}
impl<B: AsMut<[u8]>> AsMut<[u8]> for PartialBuf<B> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0.as_mut()[..]
    }
}
impl<B> Pattern for PartialBuf<B> {
    type Value = (B, usize);
}

pub struct Repeat<P>(P);
impl<P> futures::Stream for Repeat<P>
    where P: Clone + Pattern
{
    type Item = P;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(Async::Ready(Some(self.0.clone())))
    }
}
pub fn repeat<P>(pattern: P) -> Repeat<P> {
    Repeat(pattern)
}
