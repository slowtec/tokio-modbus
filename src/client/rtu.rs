use super::*;

use crate::service;

use futures::Future;
use std::io::Error;
use tokio_core::reactor::Handle;
use tokio_serial::Serial;

/// Connect to no particular Modbus slave device for sending
/// broadcast messages.
pub fn connect(handle: &Handle, serial: Serial) -> impl Future<Item = Context, Error = Error> {
    connect_slave(handle, serial, Slave::broadcast())
}

/// Connect to any kind of Modbus slave device.
pub fn connect_slave(
    handle: &Handle,
    serial: Serial,
    slave: Slave,
) -> impl Future<Item = Context, Error = Error> {
    service::rtu::connect_slave(handle, serial, slave).map(|client| Context {
        client: Box::new(client),
    })
}
