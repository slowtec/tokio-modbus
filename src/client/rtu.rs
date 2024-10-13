// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RTU client connections

use tokio::io::{AsyncRead, AsyncWrite};

use crate::service::rtu::{Client, ClientContext};

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
    let client = Client::new(transport);
    let context = ClientContext::new(client, slave);
    Context {
        client: Box::new(context),
    }
}
