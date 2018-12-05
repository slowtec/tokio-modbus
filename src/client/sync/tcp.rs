use super::{Connection, Result};

use crate::client::tcp::connect as async_connect;

use std::net::SocketAddr;
use tokio_core::reactor::Core;

pub fn connect(socket_addr: SocketAddr) -> Result<Connection> {
    let mut core = Core::new()?;
    let handle = core.handle();
    let async_connection = core.run(async_connect(socket_addr, &handle))?;
    let sync_connection = Connection {
        async_connection,
        core,
    };
    Ok(sync_connection)
}
