use super::{Context, Result};

use crate::client::rtu::connect_slave as async_connect_slave;
use crate::slave::Slave;

use tokio_serial::{Serial, SerialPortSettings};

/// Connect to no particular Modbus slave device for sending
/// broadcast messages.
pub fn connect(tty_path: &str, settings: &SerialPortSettings) -> Result<Context> {
    connect_slave(tty_path, settings, Slave::broadcast())
}

/// Connect to any kind of Modbus slave device.
pub fn connect_slave(
    tty_path: &str,
    settings: &SerialPortSettings,
    slave: Slave,
) -> Result<Context> {
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    let serial = Serial::from_path(tty_path, settings)?;
    let async_ctx = rt.block_on(async_connect_slave(serial, slave))?;
    let sync_ctx = Context { core: rt, async_ctx };
    Ok(sync_ctx)
}
