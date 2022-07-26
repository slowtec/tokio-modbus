// SPDX-FileCopyrightText: Copyright (c) 2017-2022 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![warn(rust_2018_idioms)]
#![warn(rust_2021_compatibility)]
#![warn(missing_debug_implementations)]
#![warn(unreachable_pub)]
#![cfg_attr(not(test), warn(unsafe_code))]
#![warn(clippy::all)]
#![warn(clippy::cast_lossless)]
#![warn(clippy::explicit_deref_methods)]
#![warn(clippy::explicit_into_iter_loop)]
#![warn(clippy::explicit_iter_loop)]
// TODO (v0.6): Decorate functions with #[must_use]
//#![warn(clippy::must_use_candidate)]
#![cfg_attr(not(test), warn(clippy::panic_in_result_fn))]
#![cfg_attr(not(test), warn(clippy::cast_possible_truncation))]
#![warn(rustdoc::broken_intra_doc_links)]
#![cfg_attr(not(debug_assertions), deny(warnings))]
#![cfg_attr(not(debug_assertions), deny(clippy::used_underscore_binding))]
#![doc = include_str!("../README.md")]

pub mod prelude;

pub mod client;

pub mod slave;

#[cfg(feature = "server")]
pub mod server;

mod codec;
mod frame;
mod service;
