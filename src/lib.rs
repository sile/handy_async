extern crate mio;
extern crate futures;

pub mod async;
pub mod pattern;
pub mod buffer;
pub mod net;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
