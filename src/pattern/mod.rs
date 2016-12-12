//! Patterns.
use futures::{self, Future};

pub mod read;
pub mod write;
pub mod combinators {
    //! Patterns to combinate other patterns.

    pub use super::combinators_impl::Then;
    pub use super::combinators_impl::AndThen;
    pub use super::combinators_impl::OrElse;
    pub use super::combinators_impl::Or;
    pub use super::combinators_impl::Map;
    pub use super::combinators_impl::Chain;
    pub use super::combinators_impl::IterFold;
    pub use super::combinators_impl::BE;
    pub use super::combinators_impl::LE;
    pub use super::combinators_impl::PartialBuf;
    pub use super::combinators_impl::Repeat;
}
mod combinators_impl;

/// Pattern.
pub trait Pattern: Sized {
    /// The value type associated to the pattern.
    type Value;

    /// Takes a closure which maps a `Result<Self::Value>` to a pattern, and
    /// creates a pattern which calls that closure on the evaluation result of `self`.
    fn then<F, P, E>(self, f: F) -> combinators::Then<Self, F, E>
        where F: FnOnce(Result<Self::Value, E>) -> P
    {
        combinators_impl::then(self, f)
    }

    /// Takes a closure which maps a value to a pattern, and
    /// creates a pattern which calls that closure if the evaluation of `self` was succeeded.
    fn and_then<F, P>(self, f: F) -> combinators::AndThen<Self, F>
        where F: FnOnce(Self::Value) -> P
    {
        combinators_impl::and_then(self, f)
    }

    /// Takes a closure which maps an error to a pattern, and
    /// creates a pattern which calls that closure if the evaluation of `self` failed.
    fn or_else<F, P, E>(self, f: F) -> combinators::OrElse<Self, F, E>
        where F: FnOnce(E) -> P
    {
        combinators_impl::or_else(self, f)
    }

    /// Takes a pattern `other` which will be used if the evaluation of `self` is failed.
    fn or<P>(self, other: P) -> combinators::Or<Self, P>
        where P: Pattern<Value = Self::Value>
    {
        combinators_impl::or(self, other)
    }

    /// Takes a closure which maps a value to another value, and
    /// creates a pattern which calls that closure on the evaluated value of `self`.
    fn map<F, T>(self, f: F) -> combinators::Map<Self, F>
        where F: FnOnce(Self::Value) -> T
    {
        combinators_impl::map(self, f)
    }

    /// Takes two patterns and creates a new pattern over both in sequence.
    ///
    /// In generally, using the tuple pattern `(self, P)` is more convenient way to
    /// achieve the same effect.
    fn chain<P>(self, other: P) -> combinators::Chain<Self, P>
        where P: Pattern
    {
        combinators_impl::chain(self, other)
    }

    /// Creates `Repeat` pattern to represent an infinite stream of this pattern.
    fn repeat(self) -> combinators::Repeat<Self>
        where Self: Clone
    {
        combinators_impl::repeat(self)
    }

    fn boxed<M: Matcher>(self) -> BoxPattern<M, Self::Value>
        where Self: AsyncMatch<M> + 'static,
              Self::Future: Send + 'static
    {
        let mut f = Some(move |matcher: M| self.async_match(matcher).boxed());
        BoxPattern(Box::new(move |matcher| (f.take().unwrap())(matcher)))
    }
}

use futures::BoxFuture;
use matcher::{AsyncMatch, Matcher};
use error::AsyncError;
pub struct BoxPattern<M: Matcher, T>(Box<FnMut(M) -> BoxFuture<(M, T), AsyncError<M, M::Error>>>);

impl<M: Matcher, T> Pattern for BoxPattern<M, T> {
    type Value = T;
}
impl<M: Matcher, T> AsyncMatch<M> for BoxPattern<M, T> {
    type Future = BoxFuture<(M, T), AsyncError<M, M::Error>>;
    fn async_match(mut self, matcher: M) -> Self::Future {
        (self.0)(matcher)
    }
}

/// A pattern which represents a sequence of a pattern `P`.
#[derive(Debug)]
pub struct Iter<I>(pub I);
impl<I, P> Iter<I>
    where I: Iterator<Item = P>,
          P: Pattern
{
    /// Creates `IterFold` combinator to fold the values of
    /// the patterns contained in the iterator `I`.
    pub fn fold<F, T>(self, init: T, f: F) -> combinators::IterFold<I, F, T>
        where F: Fn(T, P::Value) -> T
    {
        combinators_impl::iter_fold(self.0, f, init)
    }
}
impl<I, P> Pattern for Iter<I>
    where I: Iterator<Item = P>,
          P: Pattern
{
    type Value = ();
}

// TODO:
pub type Option<T> = ::std::option::Option<T>;

impl<P> Pattern for Option<P>
    where P: Pattern
{
    type Value = Option<P::Value>;
}

impl<T, E> Pattern for Result<T, E> {
    type Value = T;
}

macro_rules! impl_tuple_pattern{
    ($($p:ident),*) => {
        impl <$($p:Pattern),*> Pattern for ($($p),*,) {
            type Value = ($($p::Value),*);
        }
    }
}

impl Pattern for () {
    type Value = ();
}
impl_tuple_pattern!(P0, P1);
impl_tuple_pattern!(P0, P1, P2);
impl_tuple_pattern!(P0, P1, P2, P3);
impl_tuple_pattern!(P0, P1, P2, P3, P4);
impl_tuple_pattern!(P0, P1, P2, P3, P4, P5);
impl_tuple_pattern!(P0, P1, P2, P3, P4, P5, P6);
impl_tuple_pattern!(P0, P1, P2, P3, P4, P5, P6, P7);
impl_tuple_pattern!(P0, P1, P2, P3, P4, P5, P6, P7, P8);
impl_tuple_pattern!(P0, P1, P2, P3, P4, P5, P6, P7, P8, P9);

/// A pattern which represents branches in a pattern.
///
/// All branches must have the same resulting value type.
#[allow(missing_docs)]
pub enum Branch<A, B = A, C = A, D = A, E = A, F = A, G = A, H = A> {
    A(A),
    B(B),
    C(C),
    D(D),
    E(E),
    F(F),
    G(G),
    H(H),
}
impl<A, B, C, D, E, F, G, H> Pattern for Branch<A, B, C, D, E, F, G, H>
    where A: Pattern,
          B: Pattern<Value = A::Value>,
          C: Pattern<Value = A::Value>,
          D: Pattern<Value = A::Value>,
          E: Pattern<Value = A::Value>,
          F: Pattern<Value = A::Value>,
          G: Pattern<Value = A::Value>,
          H: Pattern<Value = A::Value>
{
    type Value = A::Value;
}
impl<A, B, C, D, E, F, G, H> Future for Branch<A, B, C, D, E, F, G, H>
    where A: Future,
          B: Future<Item = A::Item, Error = A::Error>,
          C: Future<Item = A::Item, Error = A::Error>,
          D: Future<Item = A::Item, Error = A::Error>,
          E: Future<Item = A::Item, Error = A::Error>,
          F: Future<Item = A::Item, Error = A::Error>,
          G: Future<Item = A::Item, Error = A::Error>,
          H: Future<Item = A::Item, Error = A::Error>
{
    type Item = A::Item;
    type Error = A::Error;
    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        match *self {
            Branch::A(ref mut f) => f.poll(),
            Branch::B(ref mut f) => f.poll(),
            Branch::C(ref mut f) => f.poll(),
            Branch::D(ref mut f) => f.poll(),
            Branch::E(ref mut f) => f.poll(),
            Branch::F(ref mut f) => f.poll(),
            Branch::G(ref mut f) => f.poll(),
            Branch::H(ref mut f) => f.poll(),
        }
    }
}

/// A trait to indicate that a pattern is partially evaluable.
pub trait AllowPartial: Sized {
    /// Indicates that this pattern is partially evaluable.
    fn allow_partial(self) -> combinators::PartialBuf<Self> {
        combinators_impl::PartialBuf(self)
    }
}

impl Pattern for Vec<u8> {
    type Value = Self;
}
impl AllowPartial for Vec<u8> {}

impl Pattern for String {
    type Value = Self;
}

/// A pattern which represents byte oriented buffer like values.
#[derive(Debug, Clone)]
pub struct Buf<B>(pub B);
impl<B> AllowPartial for Buf<B> {}
impl<B: AsRef<[u8]>> AsRef<[u8]> for Buf<B> {
    fn as_ref(&self) -> &[u8] {
        &self.0.as_ref()[..]
    }
}
impl<B: AsMut<[u8]>> AsMut<[u8]> for Buf<B> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0.as_mut()[..]
    }
}
impl<B> Pattern for Buf<B> {
    type Value = B;
}

/// A pattern which represents a window of a byte oriented buffer.
#[derive(Debug, Clone)]
pub struct Window<B> {
    inner: B,
    start: usize,
    end: usize,
}
impl<B> AllowPartial for Window<B> {}
impl<B: AsRef<[u8]> + AsMut<[u8]>> Window<B> {
    /// Makes new window.
    pub fn new(buf: B) -> Self {
        Self::new_ref(buf)
    }
}
impl<B: AsRef<[u8]>> Window<B> {
    /// Makes new window for an immutable buffer.
    pub fn new_ref(buf: B) -> Self {
        let end = buf.as_ref().len();
        Window {
            inner: buf,
            start: 0,
            end: end,
        }
    }

    /// Sets end position of the window.
    pub fn set_end(mut self, end: usize) -> Self {
        assert!(end >= self.start);
        assert!(end <= self.as_ref().len());
        self.end = end;
        self
    }
}
impl<B: AsMut<[u8]>> Window<B> {
    /// Makes new window for an mutable buffer.
    pub fn new_mut(mut buf: B) -> Self {
        let end = buf.as_mut().len();
        Window {
            inner: buf,
            start: 0,
            end: end,
        }
    }
}
impl<B> Window<B> {
    /// Returns the immutable reference of the internal buffer.
    pub fn inner_ref(&self) -> &B {
        &self.inner
    }

    /// Returns the mutable reference of the internal buffer.
    pub fn inner_mut(&mut self) -> &mut B {
        &mut self.inner
    }

    /// Converts to the internal buffer.
    pub fn into_inner(self) -> B {
        self.inner
    }

    /// Returns the start point of the window.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the end point of the window.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Converts to new window which skipped first `size` bytes of `self`.
    pub fn skip(mut self, size: usize) -> Self {
        assert!(self.start + size <= self.end);
        self.start += size;
        self
    }

    /// Converts to new window which took first `size` bytes of `self`.
    pub fn take(mut self, size: usize) -> Self {
        assert!(size <= self.end);
        assert!(self.start <= self.end - size);
        self.end -= size;
        self
    }

    /// Sets start position of the window.
    pub fn set_start(mut self, start: usize) -> Self {
        assert!(start <= self.end);
        self.start = start;
        self
    }
}
impl<B: AsRef<[u8]>> AsRef<[u8]> for Window<B> {
    fn as_ref(&self) -> &[u8] {
        &self.inner.as_ref()[self.start..self.end]
    }
}
impl<B: AsMut<[u8]>> AsMut<[u8]> for Window<B> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[self.start..self.end]
    }
}
impl<B> Pattern for Window<B> {
    type Value = Self;
}

/// A trait to indicates endianness of a pattern.
pub trait Endian: Sized {
    /// Indicates that "This is a little endian pattern".
    fn le(self) -> combinators::LE<Self> {
        combinators::LE(self)
    }

    /// Indicates that "This is a big endian pattern".
    fn be(self) -> combinators::BE<Self> {
        combinators::BE(self)
    }
}
