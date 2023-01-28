// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Synchronous RTU client example

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tokio_modbus::prelude::*;

    let tty_path = "/dev/ttyUSB0";
    let slave = Slave(0x17);

    let builder = tokio_serial::new(tty_path, 19200);

    let mut ctx = sync::rtu::connect_slave(&builder, slave)?;
    println!("Reading a sensor value");
    let rsp = ctx.read_holding_registers(0x082B, 2)?;
    println!("Sensor value is: {rsp:?}");

    Ok(())
}
