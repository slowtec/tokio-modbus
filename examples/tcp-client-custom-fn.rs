extern crate futures;
extern crate tokio_core;
extern crate tokio_modbus;

#[cfg(feature = "tcp")]
pub fn main() {
    use futures::future::Future;
    use tokio_core::reactor::Core;
    use tokio_modbus::prelude::*;

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let socket_addr = "192.168.0.222:502".parse().unwrap();

    let task = Client::connect_tcp(&socket_addr, &handle).and_then(|client| {
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
