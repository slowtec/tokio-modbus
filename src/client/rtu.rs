// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RTU client connections

use tokio::io::{AsyncRead, AsyncWrite};

use super::*;

/// Connect to no particular Modbus slave device for sending
/// broadcast messages.
pub fn attach<T>(transport: T) -> Context
where
    T: AsyncRead + AsyncWrite + Debug + Unpin + Send + 'static,
{
    attach_slave(transport, Slave::broadcast())
}

/// Connect to any kind of Modbus slave device.
pub fn attach_slave<T>(transport: T, slave: Slave) -> Context
where
    T: AsyncRead + AsyncWrite + Debug + Unpin + Send + 'static,
{
    let client = crate::service::rtu::Client::new(transport, slave);
    Context {
        client: Box::new(client),
    }
}
