use std::io;
use futures::{self, Future};

pub mod read;
pub mod write;
pub mod combinators {
    pub use super::combinators_impl::Then;
    pub use super::combinators_impl::AndThen;
    pub use super::combinators_impl::OrElse;
    pub use super::combinators_impl::Map;
    pub use super::combinators_impl::Chain;
    pub use super::combinators_impl::IterFold;
    pub use super::combinators_impl::BE;
    pub use super::combinators_impl::LE;
    pub use super::combinators_impl::PartialBuf;
    pub use super::combinators_impl::Repeat;
}
mod combinators_impl;

pub trait Pattern: Sized {
    type Value;

    fn then<F, P>(self, f: F) -> combinators::Then<Self, F>
        where F: FnOnce(io::Result<Self::Value>) -> P
    {
        combinators_impl::then(self, f)
    }
    fn and_then<F, P>(self, f: F) -> combinators::AndThen<Self, F>
        where F: FnOnce(Self::Value) -> P
    {
        combinators_impl::and_then(self, f)
    }
    fn or_else<F, P>(self, f: F) -> combinators::OrElse<Self, F>
        where F: FnOnce(io::Error) -> P
    {
        combinators_impl::or_else(self, f)
    }
    fn map<F, T>(self, f: F) -> combinators::Map<Self, F>
        where F: FnOnce(Self::Value) -> T
    {
        combinators_impl::map(self, f)
    }
    fn chain<P>(self, other: P) -> combinators::Chain<Self, P>
        where P: Pattern
    {
        combinators_impl::chain(self, other)
    }
    fn repeat(self) -> combinators::Repeat<Self>
        where Self: Clone
    {
        combinators_impl::repeat(self)
    }
}

#[derive(Debug)]
pub struct Iter<I>(pub I);
impl<I, P> Iter<I>
    where I: Iterator<Item = P>,
          P: Pattern
{
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

pub trait AllowPartial: Sized {
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

#[derive(Debug, Clone)]
pub struct Window<B> {
    inner: B,
    start: usize,
    end: usize,
}
impl<B> AllowPartial for Window<B> {}
impl<B: AsRef<[u8]>> Window<B> {
    pub fn new(buf: B) -> Self {
        let end = buf.as_ref().len();
        Window {
            inner: buf,
            start: 0,
            end: end,
        }
    }
}
impl<B: AsMut<[u8]>> Window<B> {
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
    pub fn inner_ref(&self) -> &B {
        &self.inner
    }
    pub fn inner_mut(&mut self) -> &mut B {
        &mut self.inner
    }
    pub fn into_inner(self) -> B {
        self.inner
    }
    pub fn start(&self) -> usize {
        self.start
    }
    pub fn end(&self) -> usize {
        self.end
    }
    pub fn skip(mut self, size: usize) -> Self {
        assert!(self.start + size <= self.end);
        self.start += size;
        self
    }
    pub fn take(mut self, size: usize) -> Self {
        assert!(size <= self.end);
        assert!(self.start <= self.end - size);
        self.end -= size;
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

pub trait Endian: Sized {
    fn le(self) -> combinators::LE<Self> {
        combinators::LE(self)
    }
    fn be(self) -> combinators::BE<Self> {
        combinators::BE(self)
    }
}
