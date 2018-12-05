#[cfg(all(feature = "tcp", feature = "sync"))]
pub fn main() {
    use tokio_modbus::prelude::*;

    let socket_addr = "192.168.0.222:502".parse().unwrap();
    let mut conn = sync::tcp::connect(socket_addr).unwrap();
    let buff = conn.read_input_registers(0x1000, 7).unwrap();
    println!("Response is '{:?}'", buff);
}

#[cfg(not(all(feature = "tcp", feature = "sync")))]
pub fn main() {
    println!("features `tcp` and `sync` are required to run this example");
    std::process::exit(1);
}
