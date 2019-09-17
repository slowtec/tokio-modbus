use futures::{
    future::{self, FutureResult},
    Future,
};
use std::thread;
use std::time::Duration;
use tokio_core::reactor::Core;
use tokio_service::Service;

use tokio_modbus::prelude::*;

struct MbServer;

impl Service for MbServer {
    type Request = Request;
    type Response = Response;
    type Error = std::io::Error;
    type Future = FutureResult<Self::Response, Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future {
        match req {
            Request::ReadInputRegisters(_addr, cnt) => {
                let mut registers = vec![0; cnt as usize];
                registers[2] = 0x77;
                let rsp = Response::ReadInputRegisters(registers);
                future::ok(rsp)
            }
            _ => unimplemented!(),
        }
    }
}

#[cfg(feature = "tcp")]
fn main() {
    let socket_addr = "127.0.0.1:5502".parse().unwrap();

    println!("Starting up server...");
    let _server = thread::spawn(move || {
        tcp::Server::new(socket_addr).serve(|| Ok(MbServer));
    });
    // Give the server some time for stating up
    thread::sleep(Duration::from_secs(1));

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    println!("Connecting client...");
    let task = tcp::connect(&handle, socket_addr).and_then(|ctx| {
        println!("Reading input registers...");
        ctx.read_input_registers(0x00, 7)
            .then(move |res| {
                match res {
                    Ok(rsp) => {
                        println!("The result is '{:?}'", rsp);
                    }
                    Err(err) => {
                        eprintln!("Failed to read input registers: {}", err);
                    }
                }
                Ok(ctx) // continue
            })
            .and_then(|ctx| {
                println!("Disconnecting client...");
                ctx.disconnect()
            })
    });

    core.run(task).unwrap();

    println!("Done.");
}

#[cfg(not(feature = "tcp"))]
pub fn main() {
    println!("feature `tcp` is required to run this example");
    std::process::exit(1);
}
