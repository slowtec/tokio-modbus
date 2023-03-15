// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![warn(rust_2018_idioms)]
#![warn(rust_2021_compatibility)]
#![warn(missing_debug_implementations)]
#![warn(unreachable_pub)]
#![warn(clippy::cast_lossless)]
// TODO (v0.6): Decorate functions with #[must_use]
//#![warn(clippy::must_use_candidate)]
#![cfg_attr(not(test), warn(unsafe_code))]
#![warn(clippy::pedantic)]
#![warn(clippy::clone_on_ref_ptr)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::similar_names)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::wildcard_imports)] // TODO
#![allow(clippy::missing_errors_doc)] // TODO
#![cfg_attr(not(test), warn(clippy::panic_in_result_fn))]
#![cfg_attr(not(test), warn(clippy::cast_possible_truncation))]
#![warn(rustdoc::broken_intra_doc_links)]
#![cfg_attr(not(debug_assertions), deny(warnings))]
#![cfg_attr(not(debug_assertions), deny(clippy::used_underscore_binding))]
#![doc = include_str!("../README.md")]

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
