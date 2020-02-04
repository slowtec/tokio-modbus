#[cfg(all(feature = "tcp", feature = "server"))]
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use futures::future;
    use std::{thread, time::Duration};

    use tokio_modbus::prelude::*;
    use tokio_modbus::server::Service;

    struct MbServer;

    impl Service for MbServer {
        type Request = Request;
        type Response = Response;
        type Error = std::io::Error;
        type Future = future::Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, req: Self::Request) -> Self::Future {
            match req {
                Request::ReadInputRegisters(_addr, cnt) => {
                    let mut registers = vec![0; cnt as usize];
                    registers[2] = 0x77;
                    future::ready(Ok(Response::ReadInputRegisters(registers)))
                }
                _ => unimplemented!(),
            }
        }
    }

    let socket_addr = "127.0.0.1:5502".parse().unwrap();

    println!("Starting up server...");
    let _server = thread::spawn(move || {
        tcp::Server::new(socket_addr).serve(|| Ok(MbServer));
    });
    // Give the server some time for stating up
    thread::sleep(Duration::from_secs(1));

    println!("Connecting client...");
    let mut ctx = tcp::connect(socket_addr).await?;
    println!("Reading input registers...");
    let rsp = ctx.read_input_registers(0x00, 7).await?;
    println!("The result is '{:?}'", rsp);

    Ok(())
}

#[cfg(not(all(feature = "tcp", feature = "server")))]
pub fn main() {
    println!("both `tcp` and `server` features is required to run this example");
    std::process::exit(1);
}
