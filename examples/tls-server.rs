//! TCP server example

use futures::future;
use std::net::SocketAddr;

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
    }

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
