//! A pure [Rust](https://www.rust-lang.org)
//! [Modbus](https://en.wikipedia.org/wiki/Modbus) library
//! based on [tokio](https://tokio.rs).
//!
//! Modbus is based on a [master/slave](https://en.wikipedia.org/wiki/Master/slave_(technology))
//! model.
//! To avoid confusions with the tokio terminology the master is called *client*
//! and the slave is called *server* in this library.
//!
//! ## Features
//!
//! - pure Rust library
//! - async (non-blocking)
//! - Modbus TCP
//! - Modbus RTU
//!
//! # Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! tokio-modbus = "*"
//! ```
//! If you like to use Modbus TCP only:
//!
//! ```toml
//! [dependencies]
//! tokio-modbus = { version = "*", default-features = false, features = ["tcp"] }
//! ```
//!
//! If you like to use Modbus RTU only:
//!
//! ```toml
//! [dependencies]
//! tokio-modbus = { version = "*", default-features = false, features = ["rtu"] }
//! ```
//!
//! # TCP client example
//!
//! ```rust,no_run
//! extern crate futures;
//! extern crate tokio_core;
//! extern crate tokio_modbus;
//!
//! use tokio_core::reactor::Core;
//! use futures::future::Future;
//! use tokio_modbus::{Client, TcpClient};
//!
//! pub fn main() {
//!     let mut core = Core::new().unwrap();
//!     let handle = core.handle();
//!     let addr = "192.168.0.222:502".parse().unwrap();
//!
//!     let task = TcpClient::connect(&addr, &handle).and_then(|client| {
//!         println!("Fetching the coupler ID");
//!         client
//!             .read_input_registers(0x1000, 7)
//!             .and_then(move |buff| {
//!                 println!("Response is '{:?}'", buff);
//!                 Ok(())
//!             })
//!     });
//!
//!     core.run(task).unwrap();
//! }
//! ```
//! More examples can be found in the [examples](https://github.com/slowtec/tokio-modbus/tree/master/examples) folder.
//!
//! # Protocol-Specification
//!
//! - [MODBUS Application Protocol Specification v1.1b3 (PDF)](http://modbus.org/docs/Modbus_Application_Protocol_V1_1b3.pdf)
//! - [MODBUS over serial line specification and implementation guide v1.02 (PDF)](http://modbus.org/docs/Modbus_over_serial_line_V1_02.pdf)
//! - [MODBUS Messaging on TCP/IP Implementation Guide v1.0b (PDF)](http://modbus.org/docs/Modbus_Messaging_Implementation_Guide_V1_0b.pdf)

extern crate byteorder;
extern crate bytes;
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_proto;
#[cfg(feature = "rtu")]
extern crate tokio_serial;
extern crate tokio_service;

mod frame;
mod codec;
mod proto;
mod service;
mod client;

pub use frame::*;
pub use client::Client;
#[cfg(feature = "tcp")]
pub use service::tcp::Client as TcpClient;
#[cfg(feature = "rtu")]
pub use service::rtu::Client as RtuClient;
