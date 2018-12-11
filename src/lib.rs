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
//! use tokio_core::reactor::Core;
//! use futures::future::Future;
//! use tokio_modbus::prelude::*;
//!
//! pub fn main() {
//!     let mut core = Core::new().unwrap();
//!     let handle = core.handle();
//!     let socket_addr = "192.168.0.222:502".parse().unwrap();
//!
//!     let task = tcp::connect(socket_addr, &handle).and_then(|conn| {
//!         conn
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
//! use tokio_modbus::prelude::*;
//!
//! pub fn main() {
//!     let socket_addr = "192.168.0.222:502".parse().unwrap();
//!     let mut client = client::sync::tcp::connect(socket_addr).unwrap();
//!     let data = client.read_input_registers(0x1000, 7).unwrap();
//!     println!("Response is '{:?}'", data);
//! }
//! ```
//!
//! ## RTU client
//!
//! ```rust,no_run
//! use tokio_core::reactor::Core;
//! use futures::future::Future;
//! use tokio_serial::{Serial, SerialPortSettings};
//!
//!  use tokio_modbus::prelude::*;
//!
//! pub fn main() {
//!     let mut core = Core::new().unwrap();
//!     let handle = core.handle();
//!     let tty_path = "/dev/ttyUSB0";
//!     let server_addr = 0x01;
//!
//!     let mut settings = SerialPortSettings::default();
//!     settings.baud_rate = 19200;
//!     let port = Serial::from_path_with_handle(tty_path, &settings, &handle.new_tokio_handle()).unwrap();
//!
//!     let task = rtu::connect(port, server_addr, &handle).and_then(|conn| {
//!         println!("Reading a sensor value");
//!         conn
//!             .read_holding_registers(0x082B, 2)
//!             .and_then(move |rsp| {
//!                 println!("Sensor value is: {:?}", rsp);
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

pub mod prelude;

pub mod client;
pub mod server;

mod codec;
mod frame;
mod proto;
mod service;
