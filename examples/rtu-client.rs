// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
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

    let mut conn = rtu::ClientConnection::new(port);
    println!("Reading a sensor value");
    let request = Request::ReadHoldingRegisters(0x082B, 2);
    let request_context = conn.send_request(request, slave).await?;
    let Response::ReadHoldingRegisters(value) = conn.recv_response(request_context).await?? else {
        // The response variant will always match its corresponding request variant if successful.
        unreachable!();
    };
    println!("Sensor value is: {value:?}");

    println!("Disconnecting");
    conn.disconnect().await?;

    Ok(())
}
