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

    let mut client = rtu::Client::new(transport);

    println!("Reading sensor values (request/response using the low-level API");
    let request = Request::ReadHoldingRegisters(SENSOR_ADDRESS, SENSOR_QUANTITY);
    let request_context = client.send_request(SERVER, request).await?;
    let Response::ReadHoldingRegisters(values) = client.recv_response(request_context).await??
    else {
        // The response variant will always match its corresponding request variant if successful.
        unreachable!();
    };
    println!("Sensor responded with: {values:?}");

    println!("Reading sensor values (call) using the high-level API");
    let mut client_context = client::Context::from(rtu::ClientContext::new(client, SERVER).boxed());
    let values = client_context
        .read_holding_registers(SENSOR_ADDRESS, SENSOR_QUANTITY)
        .await??;
    println!("Sensor responded with: {values:?}");

    println!("Disconnecting");
    client_context.disconnect().await?;

    Ok(())
}
