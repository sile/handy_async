use std::io::{self, Write};
use futures::Future;

use pattern::Pattern;

pub trait WriteTo<W: Write>: Pattern {
    type Future: Future<Item = (W, Self::Value), Error = (W, io::Error)>;

    fn write_to(self, writer: W) -> Self::Future;

    fn sync_write_to(self, writer: W) -> io::Result<Self::Value> {
        self.write_to(writer).map(|(_, v)| v).map_err(|(_, e)| e).wait()
    }
    // fn boxed(self) -> BoxWriteTo<W, Self::Value>
    //     where Self: Send + 'static,
    //           Self::Future: Send + 'static
    // {
    //     let mut f = Some(move |writer: W| self.write_to(writer).boxed());
    //     BoxWriteTo(Box::new(move |writer| (f.take().unwrap())(writer)))
    // }
}

// pub trait AsyncWrite: Write + Sized {
//     fn async_write<B: AsRef<[u8]>>(self, buf: B) -> WriteBytes<Self, B> {
//         WriteBytes::State(self, buf)
//     }
//     // fn async_flush(self) -> Flush<Self> {

//     // }
//     // fn async_write_all<B: AsRef<[u8]>>(self, buf: B) -> WriteAll<Self, B> {
//     // }
// }
// impl<W: Write> AsyncWrite for W {}
