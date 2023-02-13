// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus server skeletons

// TODO: Add missing documentation
#![allow(missing_docs)]

#[cfg(feature = "rtu-server")]
pub mod rtu;

#[cfg(feature = "tcp-server-unstable")]
pub mod tcp;

mod service;

pub use service::{NewService, Service};
