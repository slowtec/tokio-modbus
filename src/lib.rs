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
//! - sync (blocking)
//! - Modbus TCP
//! - Modbus RTU
//! - Client & Server
//! - Open Source (MIT/Apache-2.0)
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
//! # Examples
//!
//! ## TCP client
//!
//! ```rust,no_run
//! extern crate futures;
//! extern crate tokio_core;
//! extern crate tokio_modbus;
//!
//! use tokio_core::reactor::Core;
//! use futures::future::Future;
//! use tokio_modbus::*;
//!
//! pub fn main() {
//!     let mut core = Core::new().unwrap();
//!     let handle = core.handle();
//!     let addr = "192.168.0.222:502".parse().unwrap();
//!
//!     let task = Client::connect_tcp(&addr, &handle).and_then(|client| {
//!         client
//!             .read_input_registers(0x1000, 7)
//!             .and_then(move |data| {
//!                 println!("Response is '{:?}'", data);
//!                 Ok(())
//!             })
//!     });
//!     core.run(task).unwrap();
//! }
//! ```
//!
//! ## Sync TCP client
//!
//! ```rust,no_run
//! extern crate tokio_modbus;
//! use tokio_modbus::*;
//!
//! pub fn main() {
//!     let addr = "192.168.0.222:502".parse().unwrap();
//!     let mut client = SyncClient::connect_tcp(&addr).unwrap();
//!     let data = client.read_input_registers(0x1000, 7).unwrap();
//!     println!("Response is '{:?}'", data);
//! }
//! ```
//!
//! ## RTU client
//!
//! ```rust,no_run
//! extern crate futures;
//! extern crate tokio_core;
//! extern crate tokio_modbus;
//! extern crate tokio_serial;
//!
//! use tokio_core::reactor::Core;
//! use futures::future::Future;
//! use tokio_serial::{Serial, SerialPortSettings};
//! use tokio_modbus::*;
//!
//! pub fn main() {
//!     let mut core = Core::new().unwrap();
//!     let handle = core.handle();
//!     let tty_path = "/dev/ttyUSB0";
//!     let server_addr = 0x01;
//!
//!     let mut settings = SerialPortSettings::default();
//!     settings.baud_rate = 19200;
//!     let mut port = Serial::from_path_with_handle(tty_path, &settings, &handle.new_tokio_handle()).unwrap();
//!     port.set_exclusive(false).unwrap();
//!
//!     let task = Client::connect_rtu(port, server_addr, &handle).and_then(|client| {
//!         println!("Reading a sensor value");
//!         client
//!             .read_holding_registers(0x082B, 2)
//!             .and_then(move |res| {
//!                 println!("Sensor value is: {:?}", res);
//!                 Ok(())
//!             })
//!     });
//!
//!     core.run(task).unwrap();
//! }
//! ```
//!
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
extern crate tokio_codec;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_proto;
#[cfg(feature = "rtu")]
extern crate tokio_serial;
extern crate tokio_service;

mod client;
mod codec;
mod frame;
mod proto;
mod server;
mod service;

pub use client::*;
pub use frame::*;
pub use server::*;
