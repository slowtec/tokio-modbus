use super::*;

use crate::service;

use futures::Future;
use std::io::Error;
use tokio_core::reactor::Handle;
use tokio_serial::Serial;

/// Connect to a physical, broadcast, or custom Modbus device.
pub fn connect_device<D: Into<DeviceId>>(
    handle: &Handle,
    serial: Serial,
    device_id: D,
) -> impl Future<Item = Context, Error = Error> {
    service::rtu::connect_device(handle, serial, device_id).map(|client| Context {
        client: Box::new(client),
    })
}
