use super::*;

use crate::service;

use futures::Future;
use std::io::Error;
use tokio_core::reactor::Handle;
use tokio_io::{AsyncRead, AsyncWrite};

/// Connect to no particular Modbus slave device for sending
/// broadcast messages.
pub fn connect<T>(handle: &Handle, serial: T) -> impl Future<Item = Context, Error = Error>
where
    T: AsyncRead + AsyncWrite + 'static,
{
    connect_slave(handle, serial, Slave::broadcast())
}

/// Connect to any kind of Modbus slave device.
pub fn connect_slave<T>(
    handle: &Handle,
    serial: T,
    slave: Slave,
) -> impl Future<Item = Context, Error = Error>
where
    T: AsyncRead + AsyncWrite + 'static,
{
    service::rtu::connect_slave(handle, serial, slave).map(|client| Context {
        client: Box::new(client),
    })
}
