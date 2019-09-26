//! Connecting a Modbus RTU context

use super::*;

use crate::service;

use futures::Future;
use std::io::Error;
use tokio::io::{AsyncRead, AsyncWrite};

/// Connect to no particular Modbus slave device for sending
/// broadcast messages.
pub fn connect<T>(transport: T) -> impl Future<Output = Result<Context, Error>>
where
    T: AsyncRead + AsyncWrite + Unpin + 'static,
{
    connect_slave(transport, Slave::broadcast())
}

/// Connect to any kind of Modbus slave device.
pub fn connect_slave<T>(
    transport: T,
    slave: Slave,
) -> impl Future<Output = Result<Context, Error>>
where
    T: AsyncRead + AsyncWrite + Unpin + 'static,
{
    async move {
        let client = service::rtu::connect_slave(transport, slave).await?;

        Ok(Context {
            client: Box::new(client)
        })
    }
}
