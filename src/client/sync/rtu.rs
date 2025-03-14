// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{io, time::Duration};

use super::{block_on_with_timeout, Context};

use tokio_serial::{SerialPortBuilder, SerialStream};

use crate::Slave;

/// Connect to no particular _Modbus_ slave device for sending
/// broadcast messages.
pub fn connect(builder: &SerialPortBuilder) -> io::Result<Context> {
    connect_slave(builder, Slave::broadcast())
}

/// Connect to no particular _Modbus_ slave device for sending
/// broadcast messages with a timeout.
pub fn connect_with_timeout(
    builder: &SerialPortBuilder,
    timeout: Option<Duration>,
) -> io::Result<Context> {
    connect_slave_with_timeout(builder, Slave::broadcast(), timeout)
}

/// Connect to any kind of _Modbus_ slave device.
pub fn connect_slave(builder: &SerialPortBuilder, slave: Slave) -> io::Result<Context> {
    connect_slave_with_timeout(builder, slave, None)
}

/// Connect to any kind of _Modbus_ slave device with a timeout.
pub fn connect_slave_with_timeout(
    builder: &SerialPortBuilder,
    slave: Slave,
    timeout: Option<Duration>,
) -> io::Result<Context> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?;
    // SerialStream::open requires a runtime at least on cfg(unix).
    let serial = block_on_with_timeout(&runtime, timeout, async { SerialStream::open(builder) })?;
    let async_ctx = crate::client::rtu::attach_slave(serial, slave);
    let sync_ctx = Context {
        runtime,
        async_ctx,
        timeout,
    };
    Ok(sync_ctx)
}
