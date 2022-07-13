// SPDX-FileCopyrightText: Copyright (c) 2017-2022 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{Context, Result};

use tokio_serial::{SerialPortBuilder, SerialStream};

use crate::client::rtu::connect_slave as async_connect_slave;
use crate::slave::Slave;

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
    // SerialStream::open requires a runtime at least on cfg(unix).
    let serial = rt.block_on(async { SerialStream::open(builder) })?;
    let async_ctx = rt.block_on(async_connect_slave(serial, slave))?;
    let sync_ctx = Context {
        core: rt,
        async_ctx,
    };
    Ok(sync_ctx)
}
