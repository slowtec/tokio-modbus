use super::{Context, Result};

use crate::client::rtu::connect_slave as async_connect_slave;
use crate::slave::Slave;

use tokio_core::reactor::Core;
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
    let mut core = Core::new()?;
    let handle = core.handle();
    let serial = Serial::from_path_with_handle(tty_path, settings, &handle.new_tokio_handle())?;
    let async_ctx = core.run(async_connect_slave(&handle, serial, slave))?;
    let sync_ctx = Context { core, async_ctx };
    Ok(sync_ctx)
}
