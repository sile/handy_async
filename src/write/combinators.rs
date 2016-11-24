use std::io::{self, Write};
use futures::{Poll, Async, Future};

use pattern::{self, Pattern};
use super::WriteTo;

pub struct WriteMap<W: Write, P, F>(Option<(P::Future, F)>) where P: WriteTo<W>;
impl<W: Write, P, F, T> Future for WriteMap<W, P, F>
    where P: WriteTo<W>,
          F: FnOnce(P::Value) -> T
{
    type Item = (W, T);
    type Error = (W, io::Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (mut f, map) = self.0.take().expect("Cannot poll WriteMap twice");
        if let Async::Ready((w, v)) = f.poll()? {
            Ok(Async::Ready((w, map(v))))
        } else {
            self.0 = Some((f, map));
            Ok(Async::NotReady)
        }
    }
}
impl<W: Write, P, F, T> WriteTo<W> for pattern::Map<P, F>
    where P: WriteTo<W>,
          F: FnOnce(P::Value) -> T
{
    type Future = WriteMap<W, P, F>;
    fn write_to(self, writer: W) -> Self::Future {
        let (p, f) = self.unwrap();
        WriteMap(Some((p.write_to(writer), f)))
    }
}
