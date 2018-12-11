use super::{Context, Result};

use crate::client::tcp::{connect as async_connect, connect_device as async_connect_device};
use crate::device::DeviceId;

use std::net::SocketAddr;
use tokio_core::reactor::Core;

/// Establish a direct connection to a Modbus TCP coupler.
pub fn connect(socket_addr: SocketAddr) -> Result<Context> {
    let mut core = Core::new()?;
    let handle = core.handle();
    let async_ctx = core.run(async_connect(&handle, socket_addr))?;
    let sync_ctx = Context { core, async_ctx };
    Ok(sync_ctx)
}

/// Connect to a physical, broadcast, or custom Modbus device,
/// probably through a Modbus TCP gateway that is forwarding
/// messages to/from the corresponding unit.
pub fn connect_device<D: Into<DeviceId>>(socket_addr: SocketAddr, device_id: D) -> Result<Context> {
    let mut core = Core::new()?;
    let handle = core.handle();
    let async_ctx = core.run(async_connect_device(&handle, socket_addr, device_id))?;
    let sync_ctx = Context { core, async_ctx };
    Ok(sync_ctx)
}
