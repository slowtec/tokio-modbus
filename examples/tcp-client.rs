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
            .read_input_registers(0x1000, 7)
            .and_then(move |buff| {
                let buf: Vec<u8> = buff.iter().fold(vec![], |mut x, elem| {
                    x.push((elem & 0xff) as u8);
                    x.push((elem >> 8) as u8);
                    x
                });
                let id = String::from_utf8(buf).unwrap();
                println!("The coupler ID is '{}'", id);
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
