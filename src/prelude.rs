// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Common types and traits

///////////////////////////////////////////////////////////////////
/// Modules
///////////////////////////////////////////////////////////////////
pub use crate::client;

#[allow(missing_docs)]
#[cfg(feature = "rtu")]
pub mod rtu {
    pub use crate::client::rtu::*;
}

#[allow(missing_docs)]
#[cfg(feature = "tcp")]
pub mod tcp {
    pub use crate::client::tcp::*;
}

#[allow(missing_docs)]
#[cfg(feature = "sync")]
pub mod sync {
    pub use crate::client::sync::*;
}

///////////////////////////////////////////////////////////////////
/// Types
///////////////////////////////////////////////////////////////////
pub use crate::{Request, Response};
pub use crate::{Slave, SlaveId};

#[cfg(feature = "server")]
pub use crate::frame::SlaveRequest;

///////////////////////////////////////////////////////////////////
/// Traits
///////////////////////////////////////////////////////////////////
pub use crate::client::{Client, Reader, Writer};
pub use crate::slave::SlaveContext;

#[cfg(feature = "sync")]
pub use crate::client::sync::{Client as SyncClient, Reader as SyncReader, Writer as SyncWriter};
