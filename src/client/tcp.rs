use super::*;

use crate::service;

use futures::Future;
use std::io::Error;
use std::net::SocketAddr;
use tokio_core::reactor::Handle;

/// Establish a direct connection to a Modbus TCP coupler.
pub fn connect(
    handle: &Handle,
    socket_addr: SocketAddr,
) -> impl Future<Item = Context, Error = Error> {
    connect_slave(handle, socket_addr, Slave::tcp_device())
}

/// Connect to a physical, broadcast, or custom Modbus device,
/// probably through a Modbus TCP gateway that is forwarding
/// messages to/from the corresponding slave device.
pub fn connect_slave(
    handle: &Handle,
    socket_addr: SocketAddr,
    slave: Slave,
) -> impl Future<Item = Context, Error = Error> {
    service::tcp::connect_slave(handle, socket_addr, slave).map(|context| Context {
        client: Box::new(context),
    })
}
