//! Pattern matcher interface and its generic implementations.
use std::error;

pub use self::async_match::AsyncMatch;

pub mod futures {
    //! Futures used to match commonly used patterns.
    pub use super::async_match::{MatchMap, MatchAndThen, MatchThen, MatchChain};
    pub use super::async_match::{MatchOrElse, MatchOr, MatchOption};
    pub use super::async_match::{MatchIter, MatchIterFold, MatchExpect};
    pub use super::match_tuple::{MatchTuple3, MatchTuple4, MatchTuple5, MatchTuple6};
    pub use super::match_tuple::{MatchTuple7, MatchTuple8, MatchTuple9, MatchTuple10};
}

pub mod streams {
    //! Streams.
    pub use super::async_match::MatchStream;
}

mod async_match;
mod match_tuple;

/// A pattern matcher.
///
/// Each matcher has an intrinsic error type.
///
/// # Examples
///
/// Defines your own pattern matcher:
///
/// ```
/// # extern crate handy_async;
/// # extern crate futures;
/// use handy_async::pattern::Pattern;
/// use handy_async::matcher::{Matcher, AsyncMatch};
/// use handy_async::error::AsyncError;
/// use futures::Future;
///
/// // Defines pattern.
/// #[derive(Debug, PartialEq, Eq)]
/// enum PingPong {
///     Ping,
///     Pong,
/// }
/// impl Pattern for PingPong {
///     type Value = PingPong;
/// }
/// use PingPong::*;
///
/// // Defines pattern matcher.
/// struct PingPongMatcher;
/// impl Matcher for PingPongMatcher {
///     type Error = std::io::Error; // Dummy (This matcher never fail)
/// }
/// type PingPongError = AsyncError<PingPongMatcher, std::io::Error>;
///
/// // Implements pattern matching logic.
/// impl AsyncMatch<PingPongMatcher> for PingPong {
///     type Future = futures::Finished<(PingPongMatcher, Self), PingPongError>;
///     fn async_match(self, matcher: PingPongMatcher) -> Self::Future {
///         let response = match self {
///             Ping => Pong,
///             Pong => Ping,
///         };
///         futures::finished((matcher, response))
///     }
/// }
///
/// fn main() {
///     // NOTE: All combinator patterns are free to use.
///     let (_, value) = (Ping, Pong, Ping).async_match(PingPongMatcher).wait().ok().unwrap();
///     assert_eq!(value, (Pong, Ping, Pong));
/// }
/// ```
pub trait Matcher {
    /// The error type that may occur when matching using this matcher.
    type Error: error::Error;
}
