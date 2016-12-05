use futures::{Poll, Async, Future};

use super::Pattern;
// use super::combinators::{Map, AndThen, Then, OrElse, Or, Chain};
use super::combinators::Map;

pub trait AsyncMatch<P: Pattern, E>: Sized {
    type Future: Future<Item = (Self, P::Value), Error = (Self, E)>;
    fn async_match(self, pattern: P) -> Self::Future;
}

impl<M, P: Pattern, E, F, T> AsyncMatch<Map<P, F>, E> for M
    where F: FnOnce(P::Value) -> T,
          M: AsyncMatch<P, E>
{
    type Future = MatchMap<<M as AsyncMatch<P, E>>::Future, F>;
    fn async_match(self, pattern: Map<P, F>) -> Self::Future {
        let (p, f) = pattern.unwrap();
        MatchMap(Some((self.async_match(p), f)))
    }
}

pub struct MatchMap<P, F>(Option<(P, F)>);
impl<M, P, F, T, U> Future for MatchMap<P, F>
    where P: Future<Item = (M, T)>,
          F: FnOnce(T) -> U
{
    type Item = (M, U);
    type Error = P::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (mut p, f) = self.0.take().expect("Cannot poll MatchMap twice");
        if let Async::Ready((matcher, v)) = p.poll()? {
            Ok(Async::Ready((matcher, f(v))))
        } else {
            self.0 = Some((p, f));
            Ok(Async::NotReady)
        }
    }
}

// impl<M, E, P0, P1, F> AsyncMatch<AndThen<P0, F>, E> for M
//     where P0: Pattern,
//           P1: Pattern,
//           F: FnOnce(P0::Value) -> P1,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>
// {
//     type Future = MatchAndThen<M, E, P0, P1, F>;
//     fn async_match(self, pattern: AndThen<P0, F>) -> Self::Future {
//         let AndThen(p, f) = pattern;
//         MatchAndThen(Phase::A((self.async_match(p), f)))
//     }
// }

// pub struct MatchAndThen<M, E, P0, P1, F>(Phase<(<M as AsyncMatch<P0, E>>::Future, F),
//                                                <M as AsyncMatch<P1, E>>::Future>)
//     where P0: Pattern,
//           P1: Pattern,
//           F: FnOnce(P0::Value) -> P1,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>;

// impl<M, E, P0, P1, F> Future for MatchAndThen<M, E, P0, P1, F>
//     where P0: Pattern,
//           P1: Pattern,
//           F: FnOnce(P0::Value) -> P1,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>
// {
//     type Item = (M, P1::Value);
//     type Error = (M, E);
//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         match self.0.take() {
//             Phase::A((mut p0, f)) => {
//                 if let Async::Ready((m, v0)) = p0.poll()? {
//                     let p1 = m.async_match(f(v0));
//                     self.0 = Phase::B(p1);
//                     self.poll()
//                 } else {
//                     self.0 = Phase::A((p0, f));
//                     Ok(Async::NotReady)
//                 }
//             }
//             Phase::B(mut p1) => {
//                 if let Async::Ready((m, v1)) = p1.poll()? {
//                     Ok(Async::Ready((m, v1)))
//                 } else {
//                     self.0 = Phase::B(p1);
//                     Ok(Async::NotReady)
//                 }
//             }
//             Phase::Polled => panic!("Cannot poll MatchAndthen twice"),
//         }
//     }
// }

// impl<M, E, P0, P1, F> AsyncMatch<Then<P0, F, E>, E> for M
//     where P0: Pattern,
//           P1: Pattern,
//           F: FnOnce(Result<P0::Value, E>) -> P1,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>
// {
//     type Future = MatchThen<M, E, P0, P1, F>;
//     fn async_match(self, pattern: Then<P0, F, E>) -> Self::Future {
//         let Then(p, f, _) = pattern;
//         MatchThen(Phase::A((self.async_match(p), f)))
//     }
// }

// pub struct MatchThen<M, E, P0, P1, F>(Phase<(<M as AsyncMatch<P0, E>>::Future, F),
//                                             <M as AsyncMatch<P1, E>>::Future>)
//     where P0: Pattern,
//           P1: Pattern,
//           F: FnOnce(Result<P0::Value, E>) -> P1,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>;
// impl<M, E, P0, P1, F> Future for MatchThen<M, E, P0, P1, F>
//     where P0: Pattern,
//           P1: Pattern,
//           F: FnOnce(Result<P0::Value, E>) -> P1,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>
// {
//     type Item = (M, P1::Value);
//     type Error = (M, E);
//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         match self.0.take() {
//             Phase::A((mut p0, f)) => {
//                 match p0.poll() {
//                     Err((m, e)) => {
//                         let p1 = m.async_match(f(Err(e)));
//                         self.0 = Phase::B(p1);
//                         self.poll()
//                     }
//                     Ok(Async::Ready((m, v0))) => {
//                         let p1 = m.async_match(f(Ok(v0)));
//                         self.0 = Phase::B(p1);
//                         self.poll()
//                     }
//                     Ok(Async::NotReady) => {
//                         self.0 = Phase::A((p0, f));
//                         Ok(Async::NotReady)
//                     }
//                 }
//             }
//             Phase::B(mut p1) => {
//                 if let Async::Ready((m, v1)) = p1.poll()? {
//                     Ok(Async::Ready((m, v1)))
//                 } else {
//                     self.0 = Phase::B(p1);
//                     Ok(Async::NotReady)
//                 }
//             }
//             Phase::Polled => panic!("Cannot poll MatchThen twice"),
//         }
//     }
// }

// impl<M, E, P0, P1, F> AsyncMatch<OrElse<P0, F, E>, E> for M
//     where P0: Pattern,
//           P1: Pattern<Value = P0::Value>,
//           F: FnOnce(E) -> P1,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>
// {
//     type Future = MatchOrElse<M, E, P0, P1, F>;
//     fn async_match(self, pattern: OrElse<P0, F, E>) -> Self::Future {
//         let OrElse(p, f, _) = pattern;
//         MatchOrElse(Phase::A((self.async_match(p), f)))
//     }
// }

// pub struct MatchOrElse<M, E, P0, P1, F>(Phase<(<M as AsyncMatch<P0, E>>::Future, F),
//                                               <M as AsyncMatch<P1, E>>::Future>)
//     where P0: Pattern,
//           P1: Pattern,
//           F: FnOnce(E) -> P1,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>;
// impl<M, E, P0, P1, F> Future for MatchOrElse<M, E, P0, P1, F>
//     where P0: Pattern,
//           P1: Pattern<Value = P0::Value>,
//           F: FnOnce(E) -> P1,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>
// {
//     type Item = (M, P1::Value);
//     type Error = (M, E);
//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         match self.0.take() {
//             Phase::A((mut p0, f)) => {
//                 match p0.poll() {
//                     Err((m, e)) => {
//                         let p1 = m.async_match(f(e));
//                         self.0 = Phase::B(p1);
//                         self.poll()
//                     }
//                     Ok(Async::Ready((m, v0))) => Ok(Async::Ready((m, v0))),
//                     Ok(Async::NotReady) => {
//                         self.0 = Phase::A((p0, f));
//                         Ok(Async::NotReady)
//                     }
//                 }
//             }
//             Phase::B(mut p1) => {
//                 if let Async::Ready((m, v1)) = p1.poll()? {
//                     Ok(Async::Ready((m, v1)))
//                 } else {
//                     self.0 = Phase::B(p1);
//                     Ok(Async::NotReady)
//                 }
//             }
//             Phase::Polled => panic!("Cannot poll MatchOrElse twice"),
//         }
//     }
// }

// impl<M, E, P0, P1> AsyncMatch<Or<P0, P1>, E> for M
//     where P0: Pattern,
//           P1: Pattern<Value = P0::Value>,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>
// {
//     type Future = MatchOr<M, E, P0, P1>;
//     fn async_match(self, pattern: Or<P0, P1>) -> Self::Future {
//         let Or(p0, p1) = pattern;
//         MatchOr(Phase::A((self.async_match(p0), p1)))
//     }
// }

// pub struct MatchOr<M, E, P0, P1>(Phase<(<M as AsyncMatch<P0, E>>::Future, P1),
//                                        <M as AsyncMatch<P1, E>>::Future>)
//     where P0: Pattern,
//           P1: Pattern,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>;
// impl<M, E, P0, P1> Future for MatchOr<M, E, P0, P1>
//     where P0: Pattern,
//           P1: Pattern<Value = P0::Value>,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>
// {
//     type Item = (M, P1::Value);
//     type Error = (M, E);
//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         match self.0.take() {
//             Phase::A((mut p0, p1)) => {
//                 match p0.poll() {
//                     Err((m, _)) => {
//                         let p1 = m.async_match(p1);
//                         self.0 = Phase::B(p1);
//                         self.poll()
//                     }
//                     Ok(Async::Ready((m, v0))) => Ok(Async::Ready((m, v0))),
//                     Ok(Async::NotReady) => {
//                         self.0 = Phase::A((p0, p1));
//                         Ok(Async::NotReady)
//                     }
//                 }
//             }
//             Phase::B(mut p1) => {
//                 if let Async::Ready((m, v1)) = p1.poll()? {
//                     Ok(Async::Ready((m, v1)))
//                 } else {
//                     self.0 = Phase::B(p1);
//                     Ok(Async::NotReady)
//                 }
//             }
//             Phase::Polled => panic!("Cannot poll MatchOr twice"),
//         }
//     }
// }

// impl<M, E, P0, P1> AsyncMatch<Chain<P0, P1>, E> for M
//     where P0: Pattern,
//           P1: Pattern,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>
// {
//     type Future = MatchChain<M, E, P0, P1>;
//     fn async_match(self, pattern: Chain<P0, P1>) -> Self::Future {
//         let Chain(p0, p1) = pattern;
//         MatchChain(Phase::A((self.async_match(p0), p1)))
//     }
// }

// pub struct MatchChain<M, E, P0, P1>(Phase<(<M as AsyncMatch<P0, E>>::Future, P1),
//                                           (<M as AsyncMatch<P1, E>>::Future, P0::Value)>)
//     where P0: Pattern,
//           P1: Pattern,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>;
// impl<M, E, P0, P1> Future for MatchChain<M, E, P0, P1>
//     where P0: Pattern,
//           P1: Pattern,
//           M: AsyncMatch<P0, E> + AsyncMatch<P1, E>
// {
//     type Item = (M, (P0::Value, P1::Value));
//     type Error = (M, E);
//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         match self.0.take() {
//             Phase::A((mut p0, p1)) => {
//                 match p0.poll() {
//                     Err((m, e)) => Err((m, e)),
//                     Ok(Async::Ready((m, v0))) => {
//                         self.0 = Phase::B((m.async_match(p1), v0));
//                         self.poll()
//                     }
//                     Ok(Async::NotReady) => {
//                         self.0 = Phase::A((p0, p1));
//                         Ok(Async::NotReady)
//                     }
//                 }
//             }
//             Phase::B((mut p1, v0)) => {
//                 if let Async::Ready((m, v1)) = p1.poll()? {
//                     Ok(Async::Ready((m, (v0, v1))))
//                 } else {
//                     self.0 = Phase::B((p1, v0));
//                     Ok(Async::NotReady)
//                 }
//             }
//             Phase::Polled => panic!("Cannot poll MatchChain twice"),
//         }
//     }
// }

// #[derive(Debug)]
// enum Phase<A, B> {
//     A(A),
//     B(B),
//     Polled,
// }
// impl<A, B> Phase<A, B> {
//     pub fn take(&mut self) -> Self {
//         use std::mem;
//         mem::replace(self, Phase::Polled)
//     }
// }
