// SPDX-FileCopyrightText: Copyright (c) 2017-2026 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RTU server example with transport-level slave address filtering
//!
//! Uses [`Server::with_slave_id`] to filter requests at the transport layer,
//! so the service only receives requests addressed to this device.
//! Requests for other slave IDs are silently ignored, as required by
//! the Modbus RTU specification for devices on a shared RS-485 bus.

use std::{future, thread, time::Duration};

use tokio_modbus::{prelude::*, server::rtu::Server};

struct Service;

impl tokio_modbus::server::Service for Service {
    type Request = SlaveRequest<'static>;
    type Response = Response;
    type Exception = ExceptionCode;
    type Future = future::Ready<Result<Self::Response, Self::Exception>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        match req.request {
            Request::ReadInputRegisters(_addr, cnt) => {
                let mut registers = vec![0; cnt.into()];
                registers[2] = 0x77;
                future::ready(Ok(Response::ReadInputRegisters(registers)))
            }
            _ => future::ready(Err(ExceptionCode::IllegalFunction)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let slave = Slave(12);
    let builder = tokio_serial::new("/dev/ttyS10", 19200);
    let server_serial = tokio_serial::SerialStream::open(&builder).unwrap();

    println!("Starting up server...");
    let _server = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let server = Server::new(server_serial).with_slave_id(slave);
        let service = Service;
        rt.block_on(async {
            if let Err(err) = server.serve_forever(service).await {
                eprintln!("{err}");
            }
        });
    });

    // Give the server some time for stating up
    thread::sleep(Duration::from_secs(1));

    println!("CLIENT: Connecting client...");
    let client_serial = tokio_serial::SerialStream::open(&builder).unwrap();
    let mut ctx = rtu::attach_slave(client_serial, slave);
    println!("CLIENT: Reading input registers...");
    let rsp = ctx.read_input_registers(0x00, 7).await?;
    println!("CLIENT: The result is '{rsp:#x?}'");
    assert_eq!(rsp.unwrap(), vec![0x0, 0x0, 0x77, 0x0, 0x0, 0x0, 0x0]);

    println!("CLIENT: Reading with illegal function... (should return IllegalFunction)");
    let response = ctx.read_holding_registers(0x100, 1).await.unwrap();
    println!("CLIENT: The result is '{response:?}'");
    assert!(matches!(response, Err(ExceptionCode::IllegalFunction)));

    println!("CLIENT: Done.");

    Ok(())
}
