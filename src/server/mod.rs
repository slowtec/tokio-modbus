// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus server skeletons

// TODO: Add missing documentation
#![allow(missing_docs)]

#[cfg(feature = "rtu-server")]
pub mod rtu;

#[cfg(feature = "tcp-server")]
pub mod tcp;

#[cfg(feature = "rtu-over-tcp-server")]
pub mod rtu_over_tcp;

mod service;
pub use self::service::Service;

/// Cause for termination
#[derive(Debug, Clone)]
pub enum Terminated {
    /// The server has finished processing.
    Finished,

    /// Processing has been aborted.
    Aborted,
}
