extern crate tokio_core;
extern crate tokio_modbus;
extern crate tokio_proto;
extern crate tokio_service;
extern crate futures;

use tokio_core::reactor::Core;
use futures::future::Future;
use tokio_service::Service;
use tokio_modbus::{TcpClient, Request, Response};

pub fn main() {

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let addr = "192.168.0.222:502".parse().unwrap();

    let task = TcpClient::connect(&addr, &handle).and_then(|client| {
        println!("Fetching the coupler ID");
        client
            .call(Request::ReadInputRegisters(0x1000, 7))
            .and_then(move |response| {
                match response {
                    Response::ReadInputRegisters(buff) => {
                        let buf: Vec<u8> = buff.iter().fold(vec![], |mut x, elem| {
                            x.push((elem & 0xff) as u8);
                            x.push((elem >> 8) as u8);
                            x
                        });
                        let id = String::from_utf8(buf).unwrap();
                        println!("The coupler ID is '{}'", id);
                    }
                    _ => {
                        println!("Unexpected response: {:?}", response);
                    }
                };
                Ok(())
            })
    });

    core.run(task).unwrap();
}
