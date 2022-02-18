//! Modbus server skeletons

// TODO: Add missing documentation
#![allow(missing_docs)]

#[cfg(feature = "rtu")]
pub mod rtu;

#[cfg(feature = "tcp-server-unstable")]
pub mod tcp;

mod service;

pub use service::{NewService, Service};
