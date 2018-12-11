#[cfg(feature = "tcp")]
pub fn main() {
    use futures::Future;
    use tokio_core::reactor::Core;
    use tokio_modbus::prelude::*;

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let socket_addr = "192.168.0.222:502".parse().unwrap();

    let task = tcp::connect(&handle, socket_addr).and_then(|ctx| {
        println!("Fetching the coupler ID");
        ctx.read_input_registers(0x1000, 7).and_then(move |data| {
            let bytes: Vec<u8> = data.iter().fold(vec![], |mut x, elem| {
                x.push((elem & 0xff) as u8);
                x.push((elem >> 8) as u8);
                x
            });
            let id = String::from_utf8(bytes).unwrap();
            println!("The coupler ID is '{}'", id);
            Ok(())
        })
    });

    core.run(task).unwrap();
}

#[cfg(not(feature = "tcp"))]
pub fn main() {
    println!("feature `tcp` is required to run this example");
    std::process::exit(1);
}
