extern crate futures;
extern crate byteorder;

use std::io;

pub use self::read::ReadFrom;
pub use self::read::BoxReadFrom;
pub use self::read::{AsyncRead, ReadBytes, ReadExact, ReadFold, BytesStream};
pub use self::read::combinators::{ReadMap, ReadAndThen, ReadChain, ReadIterFold, ReadBranch};

pub mod read;
pub mod write;

pub mod pattern;

// TODO: BytesCount for read/write

pub type IoFuture<S, T> = futures::BoxFuture<(S, T), (S, io::Error)>;
