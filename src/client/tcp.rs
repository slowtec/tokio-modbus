use super::*;

use crate::device::DeviceId;
use crate::service::{self, tcp};

use futures::Future;
use std::io::Error;
use std::net::SocketAddr;
use tokio_core::reactor::Handle;

/// Establish a direct connection to a Modbus TCP coupler.
pub fn connect(
    handle: &Handle,
    socket_addr: SocketAddr,
) -> impl Future<Item = Context, Error = Error> {
    connect_device(handle, socket_addr, tcp::DIRECT_CONNECTION_DEVICE_ID)
}

/// Connect to a physical, broadcast, or custom Modbus device,
/// probably through a Modbus TCP gateway that is forwarding
/// messages to/from the corresponding slave device.
pub fn connect_device<D: Into<DeviceId>>(
    handle: &Handle,
    socket_addr: SocketAddr,
    device_id: D,
) -> impl Future<Item = Context, Error = Error> {
    service::tcp::connect_device(handle, socket_addr, device_id).map(|client| Context {
        client: Box::new(client),
    })
}
