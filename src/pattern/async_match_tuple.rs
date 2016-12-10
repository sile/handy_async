use futures::{self, Async, Poll, Future};

use super::async_match::{AsyncMatch, Matcher, MatchChain};

impl<M:Matcher> AsyncMatch<M> for () {
    type Future = futures::Finished<(M, ()), (M, M::Error)>;
    fn async_match(self, matcher: M) -> Self::Future {
        futures::finished((matcher, self))
    }
}

impl<M:Matcher, P0, P1> AsyncMatch<M> for (P0, P1)
    where P0: AsyncMatch<M>,
          P1: AsyncMatch<M>
{
    type Future = MatchChain<M, P0, P1>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (p0, p1) = self;
        p0.chain(p1).async_match(matcher)
    }
}

pub struct MatchTuple3<M, A, B, C>(Phase<(A::Future, B, C),
                                         (B::Future, C, A::Value),
                                         (C::Future, A::Value, B::Value)>)
    where M:Matcher,
          A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>;
impl<M, A, B, C> Future for MatchTuple3<M, A, B, C>
    where M:Matcher,
          A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>
{
    type Item = (M, (A::Value, B::Value, C::Value));
    type Error = (M, M::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.take() {
            Phase::A((mut future, b, c)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.0 = Phase::B((b.async_match(m), c, v));
                    self.poll()
                } else {
                    self.0 = Phase::A((future, b, c));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((mut future, c, a)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.0 = Phase::C((c.async_match(m), a, v));
                    self.poll()
                } else {
                    self.0 = Phase::B((future, c, a));
                    Ok(Async::NotReady)
                }
            }
            Phase::C((mut future, a, b)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    Ok(Async::Ready((m, (a, b, v))))
                } else {
                    self.0 = Phase::C((future, a, b));
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchTuple3 twice"),
        }
    }
}
impl<M, A, B, C> AsyncMatch<M> for (A, B, C)
    where M:Matcher,
          A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>
{
    type Future = MatchTuple3<M, A, B, C>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (a, b, c) = self;
        MatchTuple3(Phase::A((a.async_match(matcher), b, c)))
    }
}

pub struct MatchTuple4<M, A, B, C, D>
    where M:Matcher,
          A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
{
    p: Phase<(A::Future, B, C, D),
             (B::Future, C, D, A::Value),
             (C::Future, D, A::Value, B::Value),
             (D::Future, A::Value, B::Value, C::Value)>
}
impl<M, A, B, C, D> Future for MatchTuple4<M, A, B, C, D>
    where M:Matcher,
          A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
{
    type Item = (M, (A::Value, B::Value, C::Value, D::Value));
    type Error = (M, M::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.p.take() {
            Phase::A((mut future, b, c, d)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::B((b.async_match(m), c, d, v));
                    self.poll()
                } else {
                    self.p = Phase::A((future, b, c, d));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((mut future, c, d, a)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::C((c.async_match(m), d, a, v));
                    self.poll()
                } else {
                    self.p = Phase::B((future, c, d, a));
                    Ok(Async::NotReady)
                }
            }
            Phase::C((mut future, d, a, b)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::D((d.async_match(m), a, b, v));
                    self.poll()
                } else {
                    self.p = Phase::C((future, d, a, b));
                    Ok(Async::NotReady)
                }
            }
            Phase::D((mut future, a, b, c)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    Ok(Async::Ready((m, (a, b, c, v))))
                } else {
                    self.p = Phase::D((future, a, b, c));
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchTuple4 twice"),
        }
    }
}
impl<M, A, B, C, D> AsyncMatch<M> for (A, B, C, D)
    where M:Matcher,
          A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
{
    type Future = MatchTuple4<M, A, B, C, D>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (a, b, c, d) = self;
        MatchTuple4 { p: Phase::A((a.async_match(matcher), b, c, d)) }
    }
}

pub struct MatchTuple5<M, A, B, C, D, E>
    where M:Matcher,
          A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
{
    p: Phase<(A::Future, B, C, D, E),
             (B::Future, C, D, E, A::Value),
             (C::Future, D, E, A::Value, B::Value),
             (D::Future, E, A::Value, B::Value, C::Value),
             (E::Future, A::Value, B::Value, C::Value, D::Value)>
}
impl<M, A, B, C, D, E> Future for MatchTuple5<M, A, B, C, D, E>
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
{
    type Item = (M, (A::Value, B::Value, C::Value, D::Value, E::Value));
    type Error = (M, M::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.p.take() {
            Phase::A((mut future, b, c, d, e)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::B((b.async_match(m), c, d, e, v));
                    self.poll()
                } else {
                    self.p = Phase::A((future, b, c, d, e));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((mut future, c, d, e, a)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::C((c.async_match(m), d, e, a, v));
                    self.poll()
                } else {
                    self.p = Phase::B((future, c, d, e, a));
                    Ok(Async::NotReady)
                }
            }
            Phase::C((mut future, d, e, a, b)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::D((d.async_match(m), e, a, b, v));
                    self.poll()
                } else {
                    self.p = Phase::C((future, d, e, a, b));
                    Ok(Async::NotReady)
                }
            }
            Phase::D((mut future, e, a, b, c)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::E((e.async_match(m), a, b, c, v));
                    self.poll()
                } else {
                    self.p = Phase::D((future, e, a, b, c));
                    Ok(Async::NotReady)
                }
            }
            Phase::E((mut future, a, b, c, d)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    Ok(Async::Ready((m, (a, b, c, d, v))))
                } else {
                    self.p = Phase::E((future, a, b, c, d));
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchTuple5 twice"),
        }
    }
}
impl<M, A, B, C, D, E> AsyncMatch<M> for (A, B, C, D, E)
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
{
    type Future = MatchTuple5<M, A, B, C, D, E>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (a, b, c, d, e) = self;
        MatchTuple5 { p: Phase::A((a.async_match(matcher), b, c, d, e)) }
    }
}

pub struct MatchTuple6<M, A, B, C, D, E, F>
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
{
    p: Phase<(A::Future, B, C, D, E, F),
             (B::Future, C, D, E, F, A::Value),
             (C::Future, D, E, F, A::Value, B::Value),
             (D::Future, E, F, A::Value, B::Value, C::Value),
             (E::Future, F, A::Value, B::Value, C::Value, D::Value),
             (F::Future, A::Value, B::Value, C::Value, D::Value, E::Value)>
}
impl<M, A, B, C, D, E, F> Future for MatchTuple6<M, A, B, C, D, E, F>
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
{
    type Item = (M,
     (A::Value, B::Value, C::Value, D::Value, E::Value, F::Value));
    type Error = (M, M::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.p.take() {
            Phase::A((mut future, b, c, d, e, f)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::B((b.async_match(m), c, d, e, f, v));
                    self.poll()
                } else {
                    self.p = Phase::A((future, b, c, d, e, f));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((mut future, c, d, e, f, a)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::C((c.async_match(m), d, e, f, a, v));
                    self.poll()
                } else {
                    self.p = Phase::B((future, c, d, e, f, a));
                    Ok(Async::NotReady)
                }
            }
            Phase::C((mut future, d, e, f, a, b)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::D((d.async_match(m), e, f, a, b, v));
                    self.poll()
                } else {
                    self.p = Phase::C((future, d, e, f, a, b));
                    Ok(Async::NotReady)
                }
            }
            Phase::D((mut future, e, f, a, b, c)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::E((e.async_match(m), f, a, b, c, v));
                    self.poll()
                } else {
                    self.p = Phase::D((future, e, f, a, b, c));
                    Ok(Async::NotReady)
                }
            }
            Phase::E((mut future, f, a, b, c, d)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::F((f.async_match(m), a, b, c, d, v));
                    self.poll()
                } else {
                    self.p = Phase::E((future, f, a, b, c, d));
                    Ok(Async::NotReady)
                }
            }
            Phase::F((mut future, a, b, c, d, e)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    Ok(Async::Ready((m, (a, b, c, d, e, v))))
                } else {
                    self.p = Phase::F((future, a, b, c, d, e));
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchTuple6 twice"),
        }
    }
}
impl<M, A, B, C, D, E, F> AsyncMatch<M> for (A, B, C, D, E, F)
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
{
    type Future = MatchTuple6<M, A, B, C, D, E, F>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (a, b, c, d, e, f) = self;
        MatchTuple6 { p: Phase::A((a.async_match(matcher), b, c, d, e, f)) }
    }
}

pub struct MatchTuple7<M, A, B, C, D, E, F, G>
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
          G: AsyncMatch<M>,
{
    p: Phase<(A::Future, B, C, D, E, F, G),
             (B::Future, C, D, E, F, G, A::Value),
             (C::Future, D, E, F, G, A::Value, B::Value),
             (D::Future, E, F, G, A::Value, B::Value, C::Value),
             (E::Future, F, G, A::Value, B::Value, C::Value, D::Value),
             (F::Future, G, A::Value, B::Value, C::Value, D::Value, E::Value),
             (G::Future, A::Value, B::Value, C::Value, D::Value, E::Value, F::Value)>
}
impl<M, A, B, C, D, E, F, G> Future for MatchTuple7<M, A, B, C, D, E, F, G>
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
          G: AsyncMatch<M>,
{
    type Item = (M,
     (A::Value, B::Value, C::Value, D::Value, E::Value, F::Value, G::Value));
    type Error = (M, M::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.p.take() {
            Phase::A((mut future, b, c, d, e, f, g)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::B((b.async_match(m), c, d, e, f, g, v));
                    self.poll()
                } else {
                    self.p = Phase::A((future, b, c, d, e, f, g));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((mut future, c, d, e, f, g, a)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::C((c.async_match(m), d, e, f, g, a, v));
                    self.poll()
                } else {
                    self.p = Phase::B((future, c, d, e, f, g, a));
                    Ok(Async::NotReady)
                }
            }
            Phase::C((mut future, d, e, f, g, a, b)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::D((d.async_match(m), e, f, g, a, b, v));
                    self.poll()
                } else {
                    self.p = Phase::C((future, d, e, f, g, a, b));
                    Ok(Async::NotReady)
                }
            }
            Phase::D((mut future, e, f, g, a, b, c)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::E((e.async_match(m), f, g, a, b, c, v));
                    self.poll()
                } else {
                    self.p = Phase::D((future, e, f, g, a, b, c));
                    Ok(Async::NotReady)
                }
            }
            Phase::E((mut future, f, g, a, b, c, d)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::F((f.async_match(m), g, a, b, c, d, v));
                    self.poll()
                } else {
                    self.p = Phase::E((future, f, g, a, b, c, d));
                    Ok(Async::NotReady)
                }
            }
            Phase::F((mut future, g, a, b, c, d, e)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::G((g.async_match(m), a, b, c, d, e, v));
                    self.poll()
                } else {
                    self.p = Phase::F((future, g, a, b, c, d, e));
                    Ok(Async::NotReady)
                }
            }
            Phase::G((mut future, a, b, c, d, e, f)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    Ok(Async::Ready((m, (a, b, c, d, e, f, v))))
                } else {
                    self.p = Phase::G((future, a, b, c, d, e, f));
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchTuple7 twice"),
        }
    }
}
impl<M, A, B, C, D, E, F, G> AsyncMatch<M> for (A, B, C, D, E, F, G)
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
          G: AsyncMatch<M>,
{
    type Future = MatchTuple7<M, A, B, C, D, E, F, G>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (a, b, c, d, e, f, g) = self;
        MatchTuple7 { p: Phase::A((a.async_match(matcher), b, c, d, e, f, g)) }
    }
}

pub struct MatchTuple8<M, A, B, C, D, E, F, G, H>
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
          G: AsyncMatch<M>,
          H: AsyncMatch<M>,
{
    p: Phase<(A::Future, B, C, D, E, F, G, H),
             (B::Future, C, D, E, F, G, H, A::Value),
             (C::Future, D, E, F, G, H, A::Value, B::Value),
             (D::Future, E, F, G, H, A::Value, B::Value, C::Value),
             (E::Future, F, G, H, A::Value, B::Value, C::Value, D::Value),
             (F::Future, G, H, A::Value, B::Value, C::Value, D::Value, E::Value),
             (G::Future, H, A::Value, B::Value, C::Value, D::Value, E::Value, F::Value),
             (H::Future, A::Value, B::Value, C::Value, D::Value, E::Value, F::Value, G::Value)>
}
impl<M, A, B, C, D, E, F, G, H> Future for MatchTuple8<M, A, B, C, D, E, F, G, H>
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
          G: AsyncMatch<M>,
          H: AsyncMatch<M>
{
    type Item = (M,
                 (A::Value, B::Value, C::Value, D::Value, E::Value, F::Value, G::Value, H::Value));
    type Error = (M, M::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.p.take() {
            Phase::A((mut future, b, c, d, e, f, g, h)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::B((b.async_match(m), c, d, e, f, g, h, v));
                    self.poll()
                } else {
                    self.p = Phase::A((future, b, c, d, e, f, g, h));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((mut future, c, d, e, f, g, h, a)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::C((c.async_match(m), d, e, f, g, h, a, v));
                    self.poll()
                } else {
                    self.p = Phase::B((future, c, d, e, f, g, h, a));
                    Ok(Async::NotReady)
                }
            }
            Phase::C((mut future, d, e, f, g, h, a, b)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::D((d.async_match(m), e, f, g, h, a, b, v));
                    self.poll()
                } else {
                    self.p = Phase::C((future, d, e, f, g, h, a, b));
                    Ok(Async::NotReady)
                }
            }
            Phase::D((mut future, e, f, g, h, a, b, c)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::E((e.async_match(m), f, g, h, a, b, c, v));
                    self.poll()
                } else {
                    self.p = Phase::D((future, e, f, g, h, a, b, c));
                    Ok(Async::NotReady)
                }
            }
            Phase::E((mut future, f, g, h, a, b, c, d)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::F((f.async_match(m), g, h, a, b, c, d, v));
                    self.poll()
                } else {
                    self.p = Phase::E((future, f, g, h, a, b, c, d));
                    Ok(Async::NotReady)
                }
            }
            Phase::F((mut future, g, h, a, b, c, d, e)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::G((g.async_match(m), h, a, b, c, d, e, v));
                    self.poll()
                } else {
                    self.p = Phase::F((future, g, h, a, b, c, d, e));
                    Ok(Async::NotReady)
                }
            }
            Phase::G((mut future, h, a, b, c, d, e, f)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::H((h.async_match(m), a, b, c, d, e, f, v));
                    self.poll()
                } else {
                    self.p = Phase::G((future, h, a, b, c, d, e, f));
                    Ok(Async::NotReady)
                }
            }
            Phase::H((mut future, a, b, c, d, e, f, g)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    Ok(Async::Ready((m, (a, b, c, d, e, f, g, v))))
                } else {
                    self.p = Phase::H((future, a, b, c, d, e, f, g));
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchTuple8 twice"),
        }
    }
}
impl<M, A, B, C, D, E, F, G, H> AsyncMatch<M> for (A, B, C, D, E, F, G, H)
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
          G: AsyncMatch<M>,
          H: AsyncMatch<M>
{
    type Future = MatchTuple8<M, A, B, C, D, E, F, G, H>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (a, b, c, d, e, f, g, h) = self;
        MatchTuple8 { p: Phase::A((a.async_match(matcher), b, c, d, e, f, g, h)) }
    }
}

pub struct MatchTuple9<M, A, B, C, D, E, F, G, H, I>
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
          G: AsyncMatch<M>,
          H: AsyncMatch<M>,
          I: AsyncMatch<M>
{
    p: Phase<(A::Future, B, C, D, E, F, G, H, I),
             (B::Future, C, D, E, F, G, H, I, A::Value),
             (C::Future, D, E, F, G, H, I, A::Value, B::Value),
             (D::Future, E, F, G, H, I, A::Value, B::Value, C::Value),
             (E::Future, F, G, H, I, A::Value, B::Value, C::Value, D::Value),
             (F::Future, G, H, I, A::Value, B::Value, C::Value, D::Value, E::Value),
             (G::Future, H, I, A::Value, B::Value, C::Value, D::Value, E::Value, F::Value),
             (H::Future, I, A::Value, B::Value, C::Value, D::Value, E::Value, F::Value, G::Value),
             (I::Future,
              A::Value,
              B::Value,
              C::Value,
              D::Value,
              E::Value,
              F::Value,
              G::Value,
              H::Value)>,
}
impl<M, A, B, C, D, E, F, G, H, I> Future for MatchTuple9<M, A, B, C, D, E, F, G, H, I>
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
          G: AsyncMatch<M>,
          H: AsyncMatch<M>,
          I: AsyncMatch<M>
{
    type Item = (M,
     (A::Value, B::Value, C::Value, D::Value, E::Value, F::Value, G::Value, H::Value, I::Value));
    type Error = (M, M::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.p.take() {
            Phase::A((mut future, b, c, d, e, f, g, h, i)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::B((b.async_match(m), c, d, e, f, g, h, i, v));
                    self.poll()
                } else {
                    self.p = Phase::A((future, b, c, d, e, f, g, h, i));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((mut future, c, d, e, f, g, h, i, a)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::C((c.async_match(m), d, e, f, g, h, i, a, v));
                    self.poll()
                } else {
                    self.p = Phase::B((future, c, d, e, f, g, h, i, a));
                    Ok(Async::NotReady)
                }
            }
            Phase::C((mut future, d, e, f, g, h, i, a, b)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::D((d.async_match(m), e, f, g, h, i, a, b, v));
                    self.poll()
                } else {
                    self.p = Phase::C((future, d, e, f, g, h, i, a, b));
                    Ok(Async::NotReady)
                }
            }
            Phase::D((mut future, e, f, g, h, i, a, b, c)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::E((e.async_match(m), f, g, h, i, a, b, c, v));
                    self.poll()
                } else {
                    self.p = Phase::D((future, e, f, g, h, i, a, b, c));
                    Ok(Async::NotReady)
                }
            }
            Phase::E((mut future, f, g, h, i, a, b, c, d)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::F((f.async_match(m), g, h, i, a, b, c, d, v));
                    self.poll()
                } else {
                    self.p = Phase::E((future, f, g, h, i, a, b, c, d));
                    Ok(Async::NotReady)
                }
            }
            Phase::F((mut future, g, h, i, a, b, c, d, e)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::G((g.async_match(m), h, i, a, b, c, d, e, v));
                    self.poll()
                } else {
                    self.p = Phase::F((future, g, h, i, a, b, c, d, e));
                    Ok(Async::NotReady)
                }
            }
            Phase::G((mut future, h, i, a, b, c, d, e, f)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::H((h.async_match(m), i, a, b, c, d, e, f, v));
                    self.poll()
                } else {
                    self.p = Phase::G((future, h, i, a, b, c, d, e, f));
                    Ok(Async::NotReady)
                }
            }
            Phase::H((mut future, i, a, b, c, d, e, f, g)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::I((i.async_match(m), a, b, c, d, e, f, g, v));
                    self.poll()
                } else {
                    self.p = Phase::H((future, i, a, b, c, d, e, f, g));
                    Ok(Async::NotReady)
                }
            }
            Phase::I((mut future, a, b, c, d, e, f, g, h)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    Ok(Async::Ready((m, (a, b, c, d, e, f, g, h, v))))
                } else {
                    self.p = Phase::I((future, a, b, c, d, e, f, g, h));
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchTuple9 twice"),
        }
    }
}
impl<M, A, B, C, D, E, F, G, H, I> AsyncMatch<M> for (A, B, C, D, E, F, G, H, I)
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
          G: AsyncMatch<M>,
          H: AsyncMatch<M>,
          I: AsyncMatch<M>
{
    type Future = MatchTuple9<M, A, B, C, D, E, F, G, H, I>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (a, b, c, d, e, f, g, h, i) = self;
        MatchTuple9 { p: Phase::A((a.async_match(matcher), b, c, d, e, f, g, h, i)) }
    }
}

pub struct MatchTuple10<M, A, B, C, D, E, F, G, H, I, J>
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
          G: AsyncMatch<M>,
          H: AsyncMatch<M>,
          I: AsyncMatch<M>,
          J: AsyncMatch<M>
{
    p: Phase<(A::Future, B, C, D, E, F, G, H, I, J),
             (B::Future, C, D, E, F, G, H, I, J, A::Value),
             (C::Future, D, E, F, G, H, I, J, A::Value, B::Value),
             (D::Future, E, F, G, H, I, J, A::Value, B::Value, C::Value),
             (E::Future, F, G, H, I, J, A::Value, B::Value, C::Value, D::Value),
             (F::Future, G, H, I, J, A::Value, B::Value, C::Value, D::Value, E::Value),
             (G::Future, H, I, J, A::Value, B::Value, C::Value, D::Value, E::Value, F::Value),
             (H::Future,
              I,
              J,
              A::Value,
              B::Value,
              C::Value,
              D::Value,
              E::Value,
              F::Value,
              G::Value),
             (I::Future,
              J,
              A::Value,
              B::Value,
              C::Value,
              D::Value,
              E::Value,
              F::Value,
              G::Value,
              H::Value),
             (J::Future,
              A::Value,
              B::Value,
              C::Value,
              D::Value,
              E::Value,
              F::Value,
              G::Value,
              H::Value,
              I::Value)>,
}
impl<M, A, B, C, D, E, F, G, H, I, J> Future
    for MatchTuple10<M, A, B, C, D, E, F, G, H, I, J>
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
          G: AsyncMatch<M>,
          H: AsyncMatch<M>,
          I: AsyncMatch<M>,
          J: AsyncMatch<M>
{
    type Item = (M,
     (A::Value,
      B::Value,
      C::Value,
      D::Value,
      E::Value,
      F::Value,
      G::Value,
      H::Value,
      I::Value,
      J::Value));
    type Error = (M, M::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.p.take() {
            Phase::A((mut future, b, c, d, e, f, g, h, i, j)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::B((b.async_match(m), c, d, e, f, g, h, i, j, v));
                    self.poll()
                } else {
                    self.p = Phase::A((future, b, c, d, e, f, g, h, i, j));
                    Ok(Async::NotReady)
                }
            }
            Phase::B((mut future, c, d, e, f, g, h, i, j, a)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::C((c.async_match(m), d, e, f, g, h, i, j, a, v));
                    self.poll()
                } else {
                    self.p = Phase::B((future, c, d, e, f, g, h, i, j, a));
                    Ok(Async::NotReady)
                }
            }
            Phase::C((mut future, d, e, f, g, h, i, j, a, b)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::D((d.async_match(m), e, f, g, h, i, j, a, b, v));
                    self.poll()
                } else {
                    self.p = Phase::C((future, d, e, f, g, h, i, j, a, b));
                    Ok(Async::NotReady)
                }
            }
            Phase::D((mut future, e, f, g, h, i, j, a, b, c)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::E((e.async_match(m), f, g, h, i, j, a, b, c, v));
                    self.poll()
                } else {
                    self.p = Phase::D((future, e, f, g, h, i, j, a, b, c));
                    Ok(Async::NotReady)
                }
            }
            Phase::E((mut future, f, g, h, i, j, a, b, c, d)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::F((f.async_match(m), g, h, i, j, a, b, c, d, v));
                    self.poll()
                } else {
                    self.p = Phase::E((future, f, g, h, i, j, a, b, c, d));
                    Ok(Async::NotReady)
                }
            }
            Phase::F((mut future, g, h, i, j, a, b, c, d, e)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::G((g.async_match(m), h, i, j, a, b, c, d, e, v));
                    self.poll()
                } else {
                    self.p = Phase::F((future, g, h, i, j, a, b, c, d, e));
                    Ok(Async::NotReady)
                }
            }
            Phase::G((mut future, h, i, j, a, b, c, d, e, f)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::H((h.async_match(m), i, j, a, b, c, d, e, f, v));
                    self.poll()
                } else {
                    self.p = Phase::G((future, h, i, j, a, b, c, d, e, f));
                    Ok(Async::NotReady)
                }
            }
            Phase::H((mut future, i, j, a, b, c, d, e, f, g)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::I((i.async_match(m), j, a, b, c, d, e, f, g, v));
                    self.poll()
                } else {
                    self.p = Phase::H((future, i, j, a, b, c, d, e, f, g));
                    Ok(Async::NotReady)
                }
            }
            Phase::I((mut future, j, a, b, c, d, e, f, g, h)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    self.p = Phase::J((j.async_match(m), a, b, c, d, e, f, g, h, v));
                    self.poll()
                } else {
                    self.p = Phase::I((future, j, a, b, c, d, e, f, g, h));
                    Ok(Async::NotReady)
                }
            }
            Phase::J((mut future, a, b, c, d, e, f, g, h, i)) => {
                if let Async::Ready((m, v)) = future.poll()? {
                    Ok(Async::Ready((m, (a, b, c, d, e, f, g, h, i, v))))
                } else {
                    self.p = Phase::J((future, a, b, c, d, e, f, g, h, i));
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll MatchTuple10 twice"),
        }
    }
}
impl<M, A, B, C, D, E, F, G, H, I, J> AsyncMatch<M> for (A, B, C, D, E, F, G, H, I, J)
    where M:Matcher, A: AsyncMatch<M>,
          B: AsyncMatch<M>,
          C: AsyncMatch<M>,
          D: AsyncMatch<M>,
          E: AsyncMatch<M>,
          F: AsyncMatch<M>,
          G: AsyncMatch<M>,
          H: AsyncMatch<M>,
          I: AsyncMatch<M>,
          J: AsyncMatch<M>
{
    type Future = MatchTuple10<M, A, B, C, D, E, F, G, H, I, J>;
    fn async_match(self, matcher: M) -> Self::Future {
        let (a, b, c, d, e, f, g, h, i, j) = self;
        MatchTuple10 { p: Phase::A((a.async_match(matcher), b, c, d, e, f, g, h, i, j)) }
    }
}

#[derive(Debug)]
enum Phase<A, B, C = A, D = A, E = A, F = A, G = A, H = A, I = A, J = A> {
    A(A),
    B(B),
    C(C),
    D(D),
    E(E),
    F(F),
    G(G),
    H(H),
    I(I),
    J(J),
    Polled,
}
impl<A, B, C, D, E, F, G, H, I, J> Phase<A, B, C, D, E, F, G, H, I, J> {
    pub fn take(&mut self) -> Self {
        use std::mem;
        mem::replace(self, Phase::Polled)
    }
}
