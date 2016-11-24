use std::io;

pub mod read;
// pub mod write;

pub trait Pattern: Sized {
    type Value;

    fn map<F, T>(self, f: F) -> Map<Self, F>
        where F: FnOnce(Self::Value) -> T
    {
        Map {
            pattern: self,
            map: f,
        }
    }
    fn and_then<F, P>(self, f: F) -> AndThen<Self, F>
        where F: FnOnce(Self::Value) -> P
    {
        AndThen {
            pattern: self,
            and_then: f,
        }
    }
    fn then<F, P>(self, f: F) -> Then<Self, F>
        where F: FnOnce(io::Result<Self::Value>) -> P
    {
        Then {
            pattern: self,
            then: f,
        }
    }
    fn chain<P>(self, other: P) -> Chain<Self, P>
        where P: Pattern
    {
        Chain(self, other)
    }
}

pub fn iter<I, P>(iter: I) -> Iter<I>
    where I: Iterator<Item = P>,
          P: Pattern
{
    Iter(iter)
}

#[derive(Debug)]
pub struct Map<P, F> {
    pattern: P,
    map: F,
}
impl<P, F> Map<P, F> {
    pub fn unwrap(self) -> (P, F) {
        (self.pattern, self.map)
    }
}
impl<P, F, T> Pattern for Map<P, F>
    where P: Pattern,
          F: FnOnce(P::Value) -> T
{
    type Value = T;
}

#[derive(Debug)]
pub struct OrElse<P, F> {
    pattern: P,
    or_else: F,
}
impl<P, F> OrElse<P, F> {
    pub fn unwrap(self) -> (P, F) {
        (self.pattern, self.or_else)
    }
}
impl<P0, P1, F> Pattern for OrElse<P0, F>
    where P0: Pattern,
          P1: Pattern<Value = P0::Value>,
          F: FnOnce(io::Error) -> P1
{
    type Value = P1::Value;
}

#[derive(Debug)]
pub struct Then<P, F> {
    pattern: P,
    then: F,
}
impl<P, F> Then<P, F> {
    pub fn unwrap(self) -> (P, F) {
        (self.pattern, self.then)
    }
}
impl<P0, P1, F> Pattern for Then<P0, F>
    where P0: Pattern,
          P1: Pattern,
          F: FnOnce(io::Result<P0::Value>) -> P1
{
    type Value = P1::Value;
}

#[derive(Debug)]
pub struct AndThen<P, F> {
    pattern: P,
    and_then: F,
}
impl<P, F> AndThen<P, F> {
    pub fn unwrap(self) -> (P, F) {
        (self.pattern, self.and_then)
    }
}
impl<P0, P1, F> Pattern for AndThen<P0, F>
    where P0: Pattern,
          P1: Pattern,
          F: FnOnce(P0::Value) -> P1
{
    type Value = P1::Value;
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

#[derive(Debug)]
pub struct Iter<I>(pub I);
impl<I, P> Iter<I>
    where I: Iterator<Item = P>,
          P: Pattern
{
    pub fn fold<F, T>(self, init: T, f: F) -> IterFold<I, F, T>
        where F: Fn(T, P::Value) -> T
    {
        IterFold {
            iter: self.0,
            fold: f,
            acc: init,
        }
    }
}
impl<I, P> Pattern for Iter<I>
    where I: Iterator<Item = P>,
          P: Pattern
{
    type Value = ();
}
#[derive(Debug)]
pub struct IterFold<I, F, T> {
    iter: I,
    fold: F,
    acc: T,
}
impl<I, F, T> IterFold<I, F, T> {
    pub fn unwrap(self) -> (I, F, T) {
        (self.iter, self.fold, self.acc)
    }
}
impl<I, P, F, T> Pattern for IterFold<I, F, T>
    where I: Iterator<Item = P>,
          P: Pattern,
          F: Fn(T, P::Value) -> T
{
    type Value = T;
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

macro_rules! define_branch {
    ($name:ident, $p0:ident, $($p:ident,)*) => {
        #[derive(Debug, Clone)]
        pub enum $name<$p0, $($p,)*> {
            $p0($p0),
            $($p($p),)*
        }
        impl<$p0:Pattern, $($p,)*> Pattern for $name<$p0, $($p,)*>
            where $($p: Pattern<Value = $p0::Value>,)*
        {
            type Value = $p0::Value;
        }
    }
}

define_branch!(Branch2, P0, P1,);
define_branch!(Branch3, P0, P1, P2,);
define_branch!(Branch4, P0, P1, P2, P3,);
define_branch!(Branch5, P0, P1, P2, P3, P4,);
define_branch!(Branch6, P0, P1, P2, P3, P4, P5,);
define_branch!(Branch7, P0, P1, P2, P3, P4, P5, P6,);
define_branch!(Branch8, P0, P1, P2, P3, P4, P5, P6, P7,);

impl Pattern for Vec<u8> {
    type Value = Self;
}

impl Pattern for String {
    type Value = Self;
}

#[derive(Debug)]
pub struct Buf<B>(pub B);
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

#[derive(Debug)]
pub struct Window<B> {
    inner: B,
    start: usize,
    end: usize,
}
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
    fn le(self) -> LE<Self> {
        LE(self)
    }
    fn be(self) -> BE<Self> {
        BE(self)
    }
}

#[derive(Debug, Clone)]
pub struct LE<T>(pub T);

#[derive(Debug, Clone)]
pub struct BE<T>(pub T);
