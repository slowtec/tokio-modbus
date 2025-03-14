// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RTU server example

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
            Request::ReadHoldingRegisters(_, _) => {
                future::ready(Err(ExceptionCode::IllegalDataAddress))
            }
            _ => unimplemented!(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Run the following command and then copy&paste the /dev/pts/? device addresses into the code in this example:");
    println!("socat -dd pty,raw,echo=0 pty,raw,echo=0");

    println!("Connecting server");
    let server_builder = tokio_serial::new("/dev/pts/6", 19200);
    let server_serial = tokio_serial::SerialStream::open(&server_builder).unwrap();

    println!("Starting up server");
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

    println!("Connecting client");
    let client_builder = tokio_serial::new("/dev/pts/7", 19200);
    let client_serial = tokio_serial::SerialStream::open(&client_builder).unwrap();
    let mut ctx = rtu::attach(client_serial);

    println!("CLIENT: Reading input registers");
    let rsp = ctx.read_input_registers(0x00, 7).await?;
    println!("CLIENT: The result is '{rsp:#x?}'");
    assert_eq!(rsp.unwrap(), vec![0x0, 0x0, 0x77, 0x0, 0x0, 0x0, 0x0]);

    // Now we try to read with an invalid register address.
    // This should return a Modbus exception response with the code
    // IllegalDataAddress.
    println!("CLIENT: Reading nonexistent holding register address... (should return IllegalDataAddress)");
    let response = ctx.read_holding_registers(0x100, 1).await.unwrap();
    println!("CLIENT: The result is '{response:?}'");
    assert!(matches!(response, Err(ExceptionCode::IllegalDataAddress)));

    println!("CLIENT: Done.");

    Ok(())
}
