use super::{Context, Result};

use crate::client::tcp::connect_slave as async_connect_slave;
use crate::slave::Slave;

use std::net::SocketAddr;
/// Establish a direct connection to a Modbus TCP coupler.
pub fn connect(socket_addr: SocketAddr) -> Result<Context> {
    connect_slave(socket_addr, Slave::tcp_device())
}

/// Connect to any kind of Modbus slave device, probably through a Modbus TCP/RTU
/// gateway that is forwarding messages to/from the corresponding unit identified
/// by the slave parameter.
pub fn connect_slave(socket_addr: SocketAddr, slave: Slave) -> Result<Context> {
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    let async_ctx = rt.block_on(async_connect_slave(socket_addr, slave))?;
    let sync_ctx = Context { core: rt, async_ctx };
    Ok(sync_ctx)
}
