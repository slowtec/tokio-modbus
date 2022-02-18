//! TCP client connections

use super::*;

use crate::service;

use std::{future::Future, io::Error, net::SocketAddr};

/// Establish a direct connection to a Modbus TCP coupler.
pub fn connect(socket_addr: SocketAddr) -> impl Future<Output = Result<Context, Error>> {
    connect_slave(socket_addr, Slave::tcp_device())
}

/// Connect to a physical, broadcast, or custom Modbus device,
/// probably through a Modbus TCP gateway that is forwarding
/// messages to/from the corresponding slave device.
pub fn connect_slave(
    socket_addr: SocketAddr,
    slave: Slave,
) -> impl Future<Output = Result<Context, Error>> {
    let context_future = service::tcp::connect_slave(socket_addr, slave);

    async {
        let context = context_future.await?;

        Ok(Context {
            client: Box::new(context),
        })
    }
}
