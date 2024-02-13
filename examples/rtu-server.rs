// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RTU server example

use std::{future, thread, time::Duration};

use tokio_modbus::{prelude::*, server::rtu::Server};

struct Service;

impl tokio_modbus::server::Service for Service {
    type Request = SlaveRequest<'static>;
    type Future = future::Ready<Result<Response, Exception>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        match req.request {
            Request::ReadInputRegisters(_addr, cnt) => {
                let mut registers = vec![0; cnt.into()];
                registers[2] = 0x77;
                future::ready(Ok(Response::ReadInputRegisters(registers)))
            }
            Request::ReadHoldingRegisters(_, _) => {
                future::ready(Err(Exception::IllegalDataAddress))
            }
            _ => unimplemented!(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let builder = tokio_serial::new("/dev/ttyUSB0", 19200);
    let server_serial = tokio_serial::SerialStream::open(&builder).unwrap();

    println!("Starting up server...");
    let _server = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let server = Server::new(server_serial);
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
    let mut ctx = rtu::attach(client_serial);
    println!("CLIENT: Reading input registers...");
    let rsp = ctx.read_input_registers(0x00, 7).await?;
    println!("CLIENT: The result is '{rsp:#x?}'");
    assert_eq!(rsp, Ok(vec![0x0, 0x0, 0x77, 0x0, 0x0, 0x0, 0x0]));

    // Now we try to read with an invalid register address.
    // This should return a Modbus exception response with the code
    // IllegalDataAddress.
    println!("CLIENT: Reading nonexisting holding register address... (should return IllegalDataAddress)");
    let response = ctx.read_holding_registers(0x100, 1).await.unwrap();
    println!("CLIENT: The result is '{response:?}'");
    assert_eq!(response, Err(Exception::IllegalDataAddress));

    println!("CLIENT: Done.");
    Ok(())
}
