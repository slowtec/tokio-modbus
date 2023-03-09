// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{io::Result, time::Duration};

use super::{block_on_with_timeout, Context};

use tokio_serial::{SerialPortBuilder, SerialStream};

use crate::slave::Slave;

/// Connect to no particular Modbus slave device for sending
/// broadcast messages.
pub fn connect(builder: &SerialPortBuilder) -> Result<Context> {
    connect_slave(builder, Slave::broadcast())
}

/// Connect to no particular Modbus slave device for sending
/// broadcast messages with a timeout.
pub fn connect_with_timeout(
    builder: &SerialPortBuilder,
    timeout: Option<Duration>,
) -> Result<Context> {
    connect_slave_with_timeout(builder, Slave::broadcast(), timeout)
}

/// Connect to any kind of Modbus slave device.
pub fn connect_slave(builder: &SerialPortBuilder, slave: Slave) -> Result<Context> {
    connect_slave_with_timeout(builder, slave, None)
}

/// Connect to any kind of Modbus slave device with a timeout.
pub fn connect_slave_with_timeout(
    builder: &SerialPortBuilder,
    slave: Slave,
    timeout: Option<Duration>,
) -> Result<Context> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?;
    // SerialStream::open requires a runtime at least on cfg(unix).
    let serial = block_on_with_timeout(&runtime, timeout, async {
        SerialStream::open(builder).map_err(Into::into)
    })?;
    let async_ctx = crate::client::rtu::attach_slave(serial, slave);
    let sync_ctx = Context {
        runtime,
        async_ctx,
        timeout,
    };
    Ok(sync_ctx)
}
