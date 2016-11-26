handy_io
========

[![Build Status](https://travis-ci.org/sile/handy_io.svg?branch=master)](https://travis-ci.org/sile/handy_io)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

This library provides miscellaneous functionalities to help I / O operations in Rust.

`handy_io` uses [futures](https://github.com/alexcrichton/futures-rs) to achieve asynchronous I/O
and defines a lot of pattern objects to facilitate writing I/O related codes declaratively.

For example, you can write following `read_tcp_header` function to read a TCP header
defined in [RFC-793](https://www.ietf.org/rfc/rfc793.txt) asynchronously.

```rust
extern crate handy_io;
extern crate futures;

use std::io::{Read, Error};
use futures::{Future, BoxFuture};
use handy_io::io::ReadFrom;
use handy_io::pattern::{Pattern, Endian};
use handy_io::pattern::read::{U16, U32};

struct TcpHeader {
    source_port: u16,
    destination_port: u16,
    sequence_number: u32,
    acknowledgment_number: u32,
    data_offset: u8, // 4 bits
    reserved: u8, // 6 bits
    flags: u8, // 6 bits
    window: u16,
    checksum: u16,
    urgent_pointer: u16,
    option: Vec<u8>,
}

fn read_tcp_header<R: Read + Send + 'static>(reader: R) -> BoxFuture<TcpHeader, Error> {
    let pattern = (U16.be(), U16.be(), U32.be(), U32.be(),
                   U16.be(), U16.be(), U16.be(), U16.be())
        .and_then(|(src_port, dst_port, seq_num, ack_num, flags, window, checksum, urgent)| {
            let data_offset = (flags & 0b1111) as u8;
            let header = TcpHeader {
                source_port: src_port,
                destination_port: dst_port,
                sequence_number: seq_num,
                acknowledgment_number: ack_num,
                data_offset: data_offset,
                reserved: ((flags >> 4) & 0b111111) as u8,
                flags: (flags >> 10) as u8,
                window: window,
                checksum: checksum,
                urgent_pointer: urgent,
                option: Vec::new(),
            };

            let option_size = (data_offset as usize - 5) * 4;
            (Ok(header), vec![0; option_size]) // Reads additional option bytes
        })
        .map(|(mut header, option)| {
            header.option = option;
            header
        });
    pattern.read_from(reader).map(|(reader, header)| header).boxed()
}

fn main(){
    let future = read_tcp_header(std::io::stdin());
    let tcp_header = future.wait().expect("Failed to read tcp header");
}
```


Documentation
-------------

See TODO.

The documentation includes some examples.


Installation
------------

Add following lines to your `Cargo.toml`:

```toml
[dependencies]
handy_io = "0.1"
```
