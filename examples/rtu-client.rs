extern crate futures;
extern crate tokio_core;
extern crate tokio_modbus;
#[cfg(feature = "rtu")]
extern crate tokio_serial;
extern crate tokio_service;
use tokio_modbus::*;

#[cfg(feature = "rtu")]
pub fn main() {
    use tokio_core::reactor::Core;
    use futures::future::Future;
    use tokio_serial::{BaudRate, Serial, SerialPortSettings};

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let tty_path = "/dev/ttyUSB0";
    let server_addr = 0x01;

    let mut settings = SerialPortSettings::default();
    settings.baud_rate = BaudRate::Baud19200;
    let mut port = Serial::from_path(tty_path, &settings, &handle)
        .expect(&format!("Unable to open serial device '{}'", tty_path));
    port.set_exclusive(false)
        .expect("Unable to set serial port exlusive");

    let task = Client::connect_rtu(port, server_addr, &handle).and_then(|client| {
        println!("Reading a sensor value");
        client
            .read_holding_registers(0x082B, 2)
            .and_then(move |res| {
                println!("Sensor value is: {:?}", res);
                Ok(())
            })
    });

    core.run(task).unwrap();
}

#[cfg(not(feature = "rtu"))]
pub fn main() {
    println!("feature `rtu` is required to run this example");
    ::std::process::exit(1);
}
