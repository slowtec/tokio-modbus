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
//! use tokio::runtime::Runtime;
//! use futures::Future;
//! use tokio_modbus::prelude::*;
//! 
//! pub fn main() {
//!     let mut rt = tokio::runtime::Runtime::new().unwrap();
//!     let socket_addr = "192.168.0.222:502".parse().unwrap();
//!
//!     let task = async {
//!         let mut ctx = tcp::connect(socket_addr).await?;
//!         let data = ctx.read_input_registers(0x1000, 7).await?;
//!         println!("Response is '{:?}'", data);
//!         Result::<_, std::io::Error>::Ok(())
//!     };
//!     rt.block_on(task).unwrap();
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
//! use tokio::runtime::Runtime;
//! use futures::Future;
//! use tokio_serial::{Serial, SerialPortSettings};
//!
//! use tokio_modbus::prelude::*;
//!
//! pub fn main() {
//!     let mut rt = tokio::runtime::Runtime::new().unwrap();
//!     let tty_path = "/dev/ttyUSB0";
//!     let slave = Slave(0x17);
//!
//!     let mut settings = SerialPortSettings::default();
//!     settings.baud_rate = 19200;
//!     let port = Serial::from_path(tty_path, &settings).unwrap();
//!
//!     let task = async {
//!         let mut ctx = rtu::connect_slave(port, slave).await?;
//!         println!("Reading a sensor value");
//!         let rsp = ctx.read_holding_registers(0x082B, 2).await?;
//!         println!("Sensor value is: {:?}", rsp);
//!         Result::<_, std::io::Error>::Ok(())
//!     };
//!
//!     rt.block_on(task).unwrap();
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
pub mod slave;

pub use crate::service::service::{Service, NewService};

mod codec;
mod frame;
mod service;


