// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Asynchronous RTU client example

use tokio_modbus::{prelude::*, Address, Quantity, Slave};
use tokio_serial::SerialStream;

const SERIAL_PATH: &str = "/dev/ttyUSB0";

const BAUD_RATE: u32 = 19_200;

const SERVER: Slave = Slave(0x17);

const SENSOR_ADDRESS: Address = 0x082B;

const SENSOR_QUANTITY: Quantity = 2;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let builder = tokio_serial::new(SERIAL_PATH, BAUD_RATE);
    let transport = SerialStream::open(&builder).unwrap();

    let mut connection = rtu::ClientConnection::new(transport);

    println!("Reading sensor values (request/response");
    let request = Request::ReadHoldingRegisters(SENSOR_ADDRESS, SENSOR_QUANTITY);
    let request_context = connection.send_request(request, SERVER).await?;
    let Response::ReadHoldingRegisters(values) =
        connection.recv_response(request_context).await??
    else {
        // The response variant will always match its corresponding request variant if successful.
        unreachable!();
    };
    println!("Sensor responded with: {values:?}");

    println!("Reading sensor values (call");
    let mut context = rtu::client_context(connection, SERVER);
    let values = context
        .read_holding_registers(SENSOR_ADDRESS, SENSOR_QUANTITY)
        .await??;
    println!("Sensor responded with: {values:?}");

    println!("Disconnecting");
    context.disconnect().await?;

    Ok(())
}
