use std::io;
use std::marker::PhantomData;
use futures::{self, Poll, Async};

use super::{Pattern, Endian};

/// A pattern for the `then` combinator,
/// chaining a pattern on the end of another pattern regardless of its evaluation result.
///
/// This pattern is created by calling `Pattern::then` method.
#[derive(Debug)]
pub struct Then<P, F, E>(P, F, PhantomData<E>);
impl<P, F, E> Then<P, F, E> {
    #[allow(missing_docs)]
    pub fn unwrap(self) -> (P, F) {
        (self.0, self.1)
    }
}
impl<P0, P1, F, E> Pattern for Then<P0, F, E>
where
    P0: Pattern,
    P1: Pattern,
    F: FnOnce(Result<P0::Value, E>) -> P1,
{
    type Value = P1::Value;
}
pub fn then<P, F, E>(pattern: P, then: F) -> Then<P, F, E> {
    Then(pattern, then, PhantomData)
}

/// A pattern for the `and_then` combinator,
/// chaining a pattern on the end of another pattern which evaluated successfully.
///
/// This pattern is created by calling `Pattern::and_then` method.
#[derive(Debug)]
pub struct AndThen<P, F>(P, F);
impl<P, F> AndThen<P, F> {
    #[allow(missing_docs)]
    pub fn unwrap(self) -> (P, F) {
        (self.0, self.1)
    }
}
impl<P0, P1, F> Pattern for AndThen<P0, F>
where
    P0: Pattern,
    P1: Pattern,
    F: FnOnce(P0::Value) -> P1,
{
    type Value = P1::Value;
}
pub fn and_then<P, F>(pattern: P, and_then: F) -> AndThen<P, F> {
    AndThen(pattern, and_then)
}

/// A pattern for the `or` combinator,
/// chaining a pattern on the end of another pattern which evaluation fails with an error.
///
/// This pattern is created by calling `Pattern::or` method.
#[derive(Debug, Clone)]
pub struct Or<P0, P1>(P0, P1);
impl<P0, P1> Or<P0, P1> {
    #[allow(missing_docs)]
    pub fn unwrap(self) -> (P0, P1) {
        (self.0, self.1)
    }
}
impl<P0, P1> Pattern for Or<P0, P1>
where
    P0: Pattern,
    P1: Pattern<Value = P0::Value>,
{
    type Value = P1::Value;
}
pub fn or<P0, P1>(pattern0: P0, pattern1: P1) -> Or<P0, P1> {
    Or(pattern0, pattern1)
}

/// A pattern for the `or_else` combinator,
/// chaining a pattern on the end of another pattern which evaluation fails with an error.
///
/// This pattern is created by calling `Pattern::or_else` method.
#[derive(Debug)]
pub struct OrElse<P, F, E>(P, F, PhantomData<E>);
impl<P, F, E> OrElse<P, F, E> {
    #[allow(missing_docs)]
    pub fn unwrap(self) -> (P, F) {
        (self.0, self.1)
    }
}
impl<P0, P1, F, E> Pattern for OrElse<P0, F, E>
where
    P0: Pattern,
    P1: Pattern<Value = P0::Value>,
    F: FnOnce(E) -> P1,
{
    type Value = P1::Value;
}
pub fn or_else<P, F, E>(pattern: P, or_else: F) -> OrElse<P, F, E> {
    OrElse(pattern, or_else, PhantomData)
}

/// A pattern for the `map` combinator, mapping a value of a pattern to another value.
///
/// This pattern is created by calling `Pattern::map` method.
#[derive(Debug)]
pub struct Map<P, F>(P, F);
impl<P, F> Map<P, F> {
    #[allow(missing_docs)]
    pub fn unwrap(self) -> (P, F) {
        (self.0, self.1)
    }
}
impl<P, F, T> Pattern for Map<P, F>
where
    P: Pattern,
    F: FnOnce(P::Value) -> T,
{
    type Value = T;
}
pub fn map<P, F>(pattern: P, map: F) -> Map<P, F> {
    Map(pattern, map)
}

/// A pattern for the `chain` combinator,
/// chaining values of the two pattern `P0` and `P1` as a tuple value.
///
/// This pattern is created by calling `Pattern::chain` method.
#[derive(Debug, Clone)]
pub struct Chain<P0, P1>(P0, P1);
impl<P0, P1> Chain<P0, P1> {
    #[allow(missing_docs)]
    pub fn unwrap(self) -> (P0, P1) {
        (self.0, self.1)
    }
    #[allow(missing_docs)]
    pub fn inner_ref(&self) -> (&P0, &P1) {
        (&self.0, &self.1)
    }
}
impl<P0, P1> Pattern for Chain<P0, P1>
where
    P0: Pattern,
    P1: Pattern,
{
    type Value = (P0::Value, P1::Value);
}
pub fn chain<P0, P1>(p0: P0, p1: P1) -> Chain<P0, P1> {
    Chain(p0, p1)
}

/// A pattern for the `fold` combinator,
/// folding values of the patterns contained in a iterator to produce final value.
///
/// This pattern is created by calling `Iter::fold` method.
#[derive(Debug)]
pub struct IterFold<I, F, T>(I, F, T);
impl<I, F, T> IterFold<I, F, T> {
    #[allow(missing_docs)]
    pub fn unwrap(self) -> (I, F, T) {
        (self.0, self.1, self.2)
    }
    #[allow(missing_docs)]
    pub fn iter_ref(&self) -> &I {
        &self.0
    }
}
impl<I, P, F, T> Pattern for IterFold<I, F, T>
where
    I: Iterator<Item = P>,
    P: Pattern,
    F: Fn(T, P::Value) -> T,
{
    type Value = T;
}
pub fn iter_fold<I, F, T>(iter: I, fold: F, init: T) -> IterFold<I, F, T> {
    IterFold(iter, fold, init)
}

/// A pattern to indicates that "T is a little endian value".
///
/// This pattern is created by calling `Endian::le` method.
#[derive(Debug, Clone)]
pub struct LE<T>(pub T);
impl<T> Pattern for LE<T>
where
    T: Endian + Pattern,
{
    type Value = T::Value;
}

/// A pattern to indicates that "T is a big endian value".
///
/// This pattern is created by calling `Endian::be` method.
#[derive(Debug, Clone)]
pub struct BE<T>(pub T);
impl<T> Pattern for BE<T>
where
    T: Endian + Pattern,
{
    type Value = T::Value;
}

/// A pattern to indicates that "B is a partially evaluable buffer".
///
/// This pattern is created by calling `AllowPartial::allow_partial` method.
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

/// A pattern which represents infinite stream of `P`.
///
/// This pattern is created by calling `Pattern::repeat` method.
pub struct Repeat<P>(P);
impl<P> futures::Stream for Repeat<P>
where
    P: Clone + Pattern,
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

/// A pattern which represents the expected value of a pattern matching.
///
/// This pattern is created by calling `Pattern::expect_eq` method.
pub struct Expect<P: Pattern>(P, P::Value);
impl<P: Pattern> Expect<P> {
    #[allow(missing_docs)]
    pub fn unwrap(self) -> (P, P::Value) {
        (self.0, self.1)
    }
}
impl<P: Pattern> Pattern for Expect<P> {
    type Value = P::Value;
}
pub fn expect<P: Pattern>(pattern: P, expected_value: P::Value) -> Expect<P>
where
    P::Value: PartialEq,
{
    Expect(pattern, expected_value)
}

/// An unexpected value.
#[derive(Debug)]
pub struct UnexpectedValue<T>(pub T);
