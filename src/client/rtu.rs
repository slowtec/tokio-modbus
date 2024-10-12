// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RTU client connections

use tokio::io::{AsyncRead, AsyncWrite};

use crate::prelude::rtu::ClientConnection;

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
    let connection = ClientConnection::new(transport);
    client_context(connection, slave)
}

/// Creates a client/server connection.
pub fn client_context<T>(connection: ClientConnection<T>, server: Slave) -> Context
where
    T: AsyncRead + AsyncWrite + Debug + Unpin + Send + 'static,
{
    let client = crate::service::rtu::Client::new(connection, server);
    Context {
        client: Box::new(client),
    }
}
