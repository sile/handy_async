use futures::Future;

use super::Pattern;

pub trait AsyncMatch<P: Pattern, E>: Sized {
    type Future: Future<Item = (Self, P::Value), Error = (Self, E)>;
    fn async_match(self, pattern: P) -> Self::Future;
}
