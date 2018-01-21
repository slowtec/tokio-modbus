extern crate futures;
extern crate tokio_core;
extern crate tokio_modbus;
extern crate tokio_service;

#[cfg(feature = "tcp")]
pub fn main() {
    use tokio_core::reactor::Core;
    use futures::future::Future;
    use tokio_service::Service;
    use tokio_modbus::{Request, Response, TcpClient};

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let addr = "192.168.0.222:502".parse().unwrap();

    let task = TcpClient::connect(&addr, &handle).and_then(|client| {
        println!("Fetching the coupler ID");
        client
            .call(Request::Custom(0x66, vec![0x11, 0x42]))
            .and_then(move |res| {
                match res {
                    Response::Custom(f, res) => {
                        println!("Result for function {} is '{:?}'", f, res);
                    }
                    _ => {
                        panic!("unexpeted result");
                    }
                }
                Ok(())
            })
    });

    core.run(task).unwrap();
}

#[cfg(not(feature = "tcp"))]
pub fn main() {
    println!("feature `tcp` is required to run this example");
    ::std::process::exit(1);
}
