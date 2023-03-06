//! TCP server example

use futures::future;
use std::{net::SocketAddr, time::Duration};
use tokio::time::{sleep_until, Instant};

use tokio_modbus::{prelude::*, server::tls::Server, server::tls::listener};

struct Service;

impl tokio_modbus::server::Service for Service {
    type Request = Request;
    type Response = Response;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future 
    {
        match req {
            Request::ReadInputRegisters(_addr, cnt) => {
                let mut registers = vec![0; cnt.into()];
                registers[0] = 77;
                future::ready(Ok(Response::ReadInputRegisters(registers)))
            }
            _ => unimplemented!(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket_addr = "127.0.0.1:8802".parse().unwrap();

    tokio::select! {
        _ = server_context(socket_addr) => unreachable!(),
        _ = client_context(socket_addr) => println!("Exiting"),
    }

    Ok(())
}

async fn server_context(socket_addr: SocketAddr) -> anyhow::Result<()> {
    println!("Starting up server on {socket_addr}");
    let listener = listener(socket_addr, 1)?;
    let server = Server::new(listener);
    let new_service = |_socket_addr| Some(Service);
    let on_process_error = |err| {
        eprintln!("{err}");
    };
    server.serve(&new_service, on_process_error).await?;
    Ok(())
}

async fn client_context(socket_addr: SocketAddr) {
    tokio::join!(
        async {
            // Give the server some time for starting up
            tokio::time::sleep(Duration::from_secs(3)).await;

            println!("Connecting client...");
            let mut ctx = tls::connect(socket_addr).await.unwrap();
            println!("Reading input registers...");
            let response = ctx.read_input_registers(0x01, 2).await.unwrap();
            println!("The result is '{response:?}'");

            sleep_until(Instant::now() + Duration::from_secs(5)).await;

            let response = ctx.read_input_registers(0x01, 5).await.unwrap();
            println!("The result is '{response:?}'");
        },
        tokio::time::sleep(Duration::from_secs(5))
    );
}
