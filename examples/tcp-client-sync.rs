extern crate tokio_modbus;
use tokio_modbus::*;

#[cfg(all(feature = "tcp", feature = "sync"))]
pub fn main() {
    let addr = "192.168.0.222:502".parse().unwrap();
    let mut client = SyncClient::connect_tcp(&addr).unwrap();
    let buff = client.read_input_registers(0x1000, 7).unwrap();
    println!("Response is '{:?}'", buff);
}

#[cfg(not(all(feature = "tcp", feature = "sync")))]
pub fn main() {
    println!("features `tcp` and `sync` are required to run this example");
    ::std::process::exit(1);
}
