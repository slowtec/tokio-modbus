use super::{Context, Result};

use crate::client::rtu::connect_device as async_connect_device;
use crate::device::DeviceId;

use tokio_core::reactor::Core;
use tokio_serial::{Serial, SerialPortSettings};

/// Connect to a physical, broadcast, or custom Modbus device.
pub fn connect_device<D: Into<DeviceId>>(
    tty_path: &str,
    settings: &SerialPortSettings,
    device_id: D,
) -> Result<Context> {
    let mut core = Core::new()?;
    let handle = core.handle();
    let serial = Serial::from_path_with_handle(tty_path, settings, &handle.new_tokio_handle())?;
    let async_ctx = core.run(async_connect_device(&handle, serial, device_id))?;
    let sync_ctx = Context { core, async_ctx };
    Ok(sync_ctx)
}
