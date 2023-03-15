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

/// Connect to a physical, broadcast, or custom Modbus device,
/// and can enable recover strategy when modbus header mismatch,
/// when one response frame isn't arrived in specified time due
/// to tcp retransmission, in the next reading or writing previous
/// reponse frame will arrived, with this strategy the mismatched
/// response frame can be discarded. This strategy is inspired by
/// libmodbus's error recovery: 
/// https://libmodbus.org/reference/modbus_set_error_recovery/
pub async fn connect_slave_recover(socket_addr: SocketAddr, slave: Slave, max_recover_retries: usize) -> Result<Context, Error> {
    let transport = TcpStream::connect(socket_addr).await?;
    let context = attach_slave_recover(transport, slave, max_recover_retries);
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
    let client = crate::service::tcp::Client::new(transport, slave, 0);
    Context {
        client: Box::new(client),
    }
}

/// Attach a new client context to a transport connection.
///
/// The connection could either be an ordinary [`TcpStream`] or a TLS connection.
pub fn attach_slave_recover<T>(transport: T, slave: Slave, max_recover_retries: usize) -> Context
where
    T: AsyncRead + AsyncWrite + Send + Unpin + fmt::Debug + 'static,
{
    let client = crate::service::tcp::Client::new(transport, slave, max_recover_retries);
    Context {
        client: Box::new(client),
    }
}
