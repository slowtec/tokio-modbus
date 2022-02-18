//! Common types

///////////////////////////////////////////////////////////////////
/// Modules
///////////////////////////////////////////////////////////////////
pub use crate::client;

#[allow(missing_docs)]
#[cfg(feature = "sync")]
pub mod sync {
    pub use crate::client::sync::*;
}

#[allow(missing_docs)]
#[cfg(feature = "rtu")]
pub mod rtu {
    pub use crate::client::rtu::*;
}

#[allow(missing_docs)]
#[cfg(feature = "tcp")]
pub mod tcp {
    pub use crate::client::tcp::*;

    #[cfg(feature = "server")]
    pub use crate::server::*;
}

#[cfg(feature = "server")]
pub use crate::server;

///////////////////////////////////////////////////////////////////
/// Structs
///////////////////////////////////////////////////////////////////
pub use crate::frame::{Request, Response};
pub use crate::slave::{Slave, SlaveId};

///////////////////////////////////////////////////////////////////
/// Traits
///////////////////////////////////////////////////////////////////
pub use crate::client::{Client, Reader, Writer};

#[cfg(feature = "sync")]
pub use crate::client::sync::Client as SyncClient;

#[cfg(feature = "sync")]
pub use crate::client::sync::Reader as SyncReader;

#[cfg(feature = "sync")]
pub use crate::client::sync::Writer as SyncWriter;

pub use crate::slave::SlaveContext;
