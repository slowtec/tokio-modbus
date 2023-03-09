// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! TCP client connections

use std::{fmt, io::Error, net::SocketAddr};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

use super::*;

/// Establish a direct connection to a Modbus TCP coupler.
pub async fn connect(socket_addr: SocketAddr) -> Result<Context, Error> {
    connect_slave(socket_addr, Slave::tcp_device()).await
}

/// Connect to a physical, broadcast, or custom Modbus device,
/// probably through a Modbus TCP gateway that is forwarding
/// messages to/from the corresponding slave device.
pub async fn connect_slave(socket_addr: SocketAddr, slave: Slave) -> Result<Context, Error> {
    let transport = TcpStream::connect(socket_addr).await?;
    let context = attach_slave(transport, slave);
    Ok(context)
}

/// Attach a new client context to a direct transport connection.
///
/// The connection could either be an ordinary [`TcpStream`] or a TLS connection.
pub fn attach<T>(transport: T) -> Context
where
    T: AsyncRead + AsyncWrite + Send + Unpin + fmt::Debug + 'static,
{
    attach_slave(transport, Slave::tcp_device())
}

/// Attach a new client context to a transport connection.
///
/// The connection could either be an ordinary [`TcpStream`] or a TLS connection.
pub fn attach_slave<T>(transport: T, slave: Slave) -> Context
where
    T: AsyncRead + AsyncWrite + Send + Unpin + fmt::Debug + 'static,
{
    let client = crate::service::tcp::Client::new(transport, slave);
    Context {
        client: Box::new(client),
    }
}
