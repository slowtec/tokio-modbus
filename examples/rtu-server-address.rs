// SPDX-FileCopyrightText: Copyright (c) 2017-2022 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RTU server example with slave address filtering and optional response

use std::{thread, time::Duration};

use futures::future;

use tokio_modbus::{prelude::*, server::rtu::Server};

struct Service {
    slave: Slave,
}

impl tokio_modbus::server::Service for Service {
    type Request = SlaveRequest<'static>;
    type Response = Option<Response>;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        if req.slave != self.slave.into() {
            return future::ready(Ok(None));
        }
        match req.request {
            Request::ReadInputRegisters(_addr, cnt) => {
                let mut registers = vec![0; cnt.into()];
                registers[2] = 0x77;
                future::ready(Ok(Some(Response::ReadInputRegisters(registers))))
            }
            _ => unimplemented!(),
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
        let server = Server::new(server_serial);
        let service = Service { slave };
        rt.block_on(async {
            if let Err(err) = server.serve_forever(service).await {
                eprintln!("{err}");
            }
        });
    });

    // Give the server some time for stating up
    thread::sleep(Duration::from_secs(1));

    println!("Connecting client...");
    let client_serial = tokio_serial::SerialStream::open(&builder).unwrap();
    let mut ctx = rtu::attach_slave(client_serial, slave);
    println!("Reading input registers...");
    let rsp = ctx.read_input_registers(0x00, 7).await?;
    println!("The result is '{rsp:#x?}'"); // The result is '[0x0,0x0,0x77,0x0,0x0,0x0,0x0,]'

    Ok(())
}
