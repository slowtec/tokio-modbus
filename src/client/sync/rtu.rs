use super::{Context, Result};

use crate::client::rtu::connect as async_connect;

use tokio_core::reactor::Core;
use tokio_serial::{Serial, SerialPortSettings};

pub fn connect(tty_path: &str, settings: &SerialPortSettings, address: u8) -> Result<Context> {
    let mut core = Core::new()?;
    let handle = core.handle();
    let serial = Serial::from_path_with_handle(tty_path, settings, &handle.new_tokio_handle())?;
    let async_ctx = core.run(async_connect(serial, address, &handle))?;
    let sync_ctx = Context {
        async_ctx,
        core,
    };
    Ok(sync_ctx)
}
