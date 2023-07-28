// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![warn(rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]
#![warn(rust_2018_idioms)]
#![warn(rust_2021_compatibility)]
#![warn(missing_debug_implementations)]
//#![warn(missing_docs)] // TODO
#![warn(unreachable_pub)]
#![warn(unsafe_code)]
#![warn(rustdoc::broken_intra_doc_links)]
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

mod frame;
pub use self::frame::{Address, FunctionCode, Quantity, Request, Response};

mod service;

#[cfg(feature = "server")]
pub mod server;
