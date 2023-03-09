// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Asynchronous RTU client example

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tokio_serial::SerialStream;

    use tokio_modbus::prelude::*;

    let tty_path = "/dev/ttyUSB0";
    let slave = Slave(0x17);

    let builder = tokio_serial::new(tty_path, 19200);
    let port = SerialStream::open(&builder).unwrap();

    let mut ctx = rtu::attach_slave(port, slave);
    println!("Reading a sensor value");
    let rsp = ctx.read_holding_registers(0x082B, 2).await?;
    println!("Sensor value is: {rsp:?}");

    Ok(())
}
