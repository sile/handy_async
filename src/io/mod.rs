pub use self::read::{AsyncRead, ReadFrom, BoxReadFrom};
pub use self::write::{AsyncWrite, WriteTo, BoxWriteTo};

pub mod futures {
    pub type IoFuture<S, T> = ::futures::BoxFuture<(S, T), (S, ::std::io::Error)>;

    pub use super::read::{LossyReadFrom, ReadBytes, ReadNonEmpty, ReadExact, ReadFold};
    pub use super::read::primitives::{ReadBuf, ReadPartialBuf, ReadString, ReadEos};
    pub use super::read::combinators::{ReadThen, ReadAndThen, ReadOrElse, ReadMap, ReadChain};
    pub use super::read::combinators::{ReadIter, ReadIterFold, ReadOption, ReadResult, ReadBranch};

    pub use super::write::{WriteBytes, WriteAll, Flush, LossyWriteTo};
    pub use super::write::primitives::{WriteBuf, WritePartialBuf};
    pub use super::write::combinators::{WriteThen, WriteAndThen, WriteOrElse, WriteMap};
    pub use super::write::combinators::{WriteChain, WriteBranch, WriteIterFold, WriteIter};
    pub use super::write::combinators::{WriteResult, WriteOption};
}

pub mod streams {
    pub use super::read::ReadStream;
    pub use super::write::WriteStream;
}

mod read;
mod write;
mod common;
