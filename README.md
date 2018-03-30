handy_async
===========

[![Crates.io: handy_async](http://meritbadge.herokuapp.com/handy_async)](https://crates.io/crates/handy_async)
[![Documentation](https://docs.rs/handy_async/badge.svg)](https://docs.rs/handy_async)
[![Build Status](https://travis-ci.org/sile/handy_async.svg?branch=master)](https://travis-ci.org/sile/handy_async)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

**NOTICE**: [bytecodec](https://github.com/sile/bytecodec) is more recommended for implementing protocols that support asynchronous I/O.

---

This library provides miscellaneous functionalities to help asynchronous operations in Rust.

[Documentation](https://docs.rs/handy_async)

`handy_async` uses [futures](https://github.com/alexcrichton/futures-rs) to
achieve asynchronous operations (mainly I/O related operations)
and defines a lot of pattern objects to facilitate writing declarative code.

For example, you can write a function to read a TCP header
defined in [RFC-793](https://www.ietf.org/rfc/rfc793.txt) asynchronously as following.

```rust
extern crate handy_async;
extern crate futures;

use std::io::{Read, Error};
use futures::{Future, BoxFuture};
use handy_async::io::ReadFrom;
use handy_async::pattern::{Pattern, Endian};
use handy_async::pattern::read::{U16, U32};

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
    pattern.read_from(reader).map(|(reader, header)| header).map_err(|e| e.into_error()).boxed()
}
```

Installation
------------

Add following lines to your `Cargo.toml`:

```toml
[dependencies]
handy_async = "0.2"
```
