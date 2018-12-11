use super::{Context, Result};

use crate::client::tcp::connect as async_connect;

use std::net::SocketAddr;
use tokio_core::reactor::Core;

pub fn connect(socket_addr: SocketAddr) -> Result<Context> {
    let mut core = Core::new()?;
    let handle = core.handle();
    let async_ctx = core.run(async_connect(&handle, socket_addr))?;
    let sync_ctx = Context { async_ctx, core };
    Ok(sync_ctx)
}
