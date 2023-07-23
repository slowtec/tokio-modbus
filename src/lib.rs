// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![warn(rust_2018_idioms)]
#![warn(rust_2021_compatibility)]
#![warn(missing_debug_implementations)]
#![warn(unreachable_pub)]
#![cfg_attr(not(test), warn(unsafe_code))]
#![warn(clippy::pedantic)]
#![warn(clippy::clone_on_ref_ptr)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::similar_names)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::wildcard_imports)] // TODO
#![allow(clippy::missing_errors_doc)] // TODO
#![warn(rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]

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
