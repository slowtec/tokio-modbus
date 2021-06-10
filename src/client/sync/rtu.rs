use super::{Context, Result};

use crate::client::rtu::connect_slave as async_connect_slave;
use crate::slave::Slave;

use serial_io::{AsyncSerial, SerialPortBuilder};

/// Connect to no particular Modbus slave device for sending
/// broadcast messages.
pub fn connect(builder: &SerialPortBuilder) -> Result<Context> {
    connect_slave(builder, Slave::broadcast())
}

/// Connect to any kind of Modbus slave device.
pub fn connect_slave(builder: &SerialPortBuilder, slave: Slave) -> Result<Context> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;
    let serial = AsyncSerial::from_builder(builder).unwrap();
    let async_ctx = rt.block_on(async_connect_slave(serial, slave))?;
    let sync_ctx = Context {
        core: rt,
        async_ctx,
    };
    Ok(sync_ctx)
}
