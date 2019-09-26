use std::thread;
use std::time::Duration;

use tokio_modbus::Service;

use tokio_modbus::prelude::*;

struct MbServer;

impl Service for MbServer {
    type Request = Request;
    type Response = Response;
    type Error = std::io::Error;

    fn call(&self, req: Self::Request) -> Self::Response {
        match req {
            Request::ReadInputRegisters(_addr, cnt) => {
                let mut registers = vec![0; cnt as usize];
                registers[2] = 0x77;
                let rsp = Response::ReadInputRegisters(registers);
                rsp
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

    let mut rt = tokio::runtime::Runtime::new().unwrap();

    println!("Connecting client...");
    let task = async {
        let mut ctx = tcp::connect(socket_addr).await?;
        println!("Reading input registers...");
        let rsp = ctx.read_input_registers(0x00, 7).await?;
        println!("The result is '{:?}'", rsp);

        Result::<_, std::io::Error>::Ok(())
    };

    rt.block_on(task).unwrap();
}

#[cfg(not(feature = "tcp"))]
pub fn main() {
    println!("feature `tcp` is required to run this example");
    std::process::exit(1);
}
