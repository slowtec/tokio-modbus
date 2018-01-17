//! A pure [Rust](https://www.rust-lang.org)
//! [Modbus](https://en.wikipedia.org/wiki/Modbus) library
//! based on [tokio](https://tokio.rs).
//!
//! Modbus is based on a [master/slave](https://en.wikipedia.org/wiki/Master/slave_(technology))
//! model.
//! To avoid confusions with the tokio terminology the master is called *client*
//! and the slave is called *server* in this library.
//!
//! ## Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! tokio-modbus = "*"
//! ```

extern crate bytes;
extern crate byteorder;
extern crate futures;
extern crate tokio_io;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;
#[cfg(feature = "rtu")]
extern crate tokio_serial;

mod frame;
mod codec;
mod proto;
mod service;
mod client;

pub use frame::*;
pub use client::Client;
pub use service::tcp::Client as TcpClient;
pub use service::rtu::Client as RtuClient;
