// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![doc = include_str!("../README.md")]
// Opt-in for allowed-by-default lints (in alphabetical order)
// See also: <https://doc.rust-lang.org/rustc/lints>
#![warn(future_incompatible)]
#![warn(let_underscore)]
#![warn(missing_debug_implementations)]
//#![warn(missing_docs)] // TODO
#![warn(rust_2018_idioms)]
#![warn(rust_2021_compatibility)]
#![warn(unreachable_pub)]
#![warn(unsafe_code)]
#![warn(unused)]
// Clippy lints
#![warn(clippy::pedantic)]
// Additional restrictions
#![warn(clippy::clone_on_ref_ptr)]
#![warn(clippy::self_named_module_files)]
// Exceptions
#![allow(clippy::enum_glob_use)]
#![allow(clippy::similar_names)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::wildcard_imports)] // TODO
#![allow(clippy::missing_errors_doc)] // TODO

/// Re-export the `bytes` crate
///
/// Needed to prevent version conflicts with types that are exposed by the public API.
///
/// Used by [`Response::Custom`].
pub use bytes;

pub mod prelude;

pub mod client;

pub mod slave;
pub use self::slave::{Slave, SlaveId};

mod codec;

mod error;
pub use self::error::ResponseError;

mod frame;
pub use self::frame::{
    Address, Exception, ExceptionResponse, FunctionCode, Quantity, Request, Response,
};

/// Specialized [`std::result::Result`] type for type-checked responses of the _Modbus_ client API.
///
/// The payload is generic over the response type.
///
/// This [`Result`] type contains 2 layers of errors.
///
/// 1. [`std::io::Error`]: An error occurred while performing I/O operations.
/// 2. [`ResponseError`]: An error occurred on the _Modbus_ server or when matching the received response with the request.
pub type Result<T> = std::io::Result<std::result::Result<T, ResponseError>>;

/// Result of a generic [`Request`].
pub type ResponseResult = Result<Response>;

mod service;

#[cfg(feature = "server")]
pub mod server;
