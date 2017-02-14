//! Future related functionalities.
use futures::{Future, IntoFuture};

/// An extention of the `Future` trait.
pub trait FutureExt: Future + Sized {
    /// Polls both AAA and BBB, will select one which is available first.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate futures;
    /// # extern crate handy_async;
    /// use futures::{Future, empty, failed};
    /// use futures::future::Either;
    /// use handy_async::future::FutureExt;
    ///
    /// # fn main() {
    /// let future = empty::<(), ()>().select_either(Ok(10) as Result<_, ()>);
    /// if let Ok(Either::B((_, 10))) = future.wait() {
    /// } else {
    ///     panic!();
    /// }
    ///
    /// let future = failed::<(), usize>(10).select_either(empty::<(), ()>());
    /// if let Err(Either::A((10, _))) = future.wait() {
    /// } else {
    ///     panic!();
    /// }
    /// # }
    /// ```
    fn select_either<B>(self, other: B) -> futures::SelectEither<Self, B::Future>
        where B: IntoFuture
    {
        impls::select_either(self, other.into_future())
    }
}
impl<T: Future> FutureExt for T {}

pub mod futures {
    //! `Future` trait implementations.
    pub use super::impls::SelectEither;
}

mod impls {
    use futures::{Future, Poll, Async};
    use futures::future::Either;

    pub fn select_either<A: Future, B: Future>(a: A, b: B) -> SelectEither<A, B> {
        SelectEither(Some((a, b)))
    }

    /// This future polls both AAA and BBB, will select one which is available first.
    ///
    /// This is created by calling `FutureExt::select_either` method.
    pub struct SelectEither<A, B>(Option<(A, B)>);
    impl<A, B> Future for SelectEither<A, B>
        where A: Future,
              B: Future
    {
        type Item = Either<(A::Item, B), (A, B::Item)>;
        type Error = Either<(A::Error, B), (A, B::Error)>;
        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            let (mut a, mut b) = self.0.take().expect("Cannot poll SelectEither twice");
            match a.poll() {
                Err(e) => return Err(Either::A((e, b))),
                Ok(Async::Ready(v)) => return Ok(Async::Ready(Either::A((v, b)))),
                Ok(Async::NotReady) => {}
            }
            match b.poll() {
                Err(e) => return Err(Either::B((a, e))),
                Ok(Async::Ready(v)) => return Ok(Async::Ready(Either::B((a, v)))),
                Ok(Async::NotReady) => {}
            }
            self.0 = Some((a, b));
            Ok(Async::NotReady)
        }
    }
}
