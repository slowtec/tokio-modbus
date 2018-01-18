extern crate futures;
extern crate tokio_core;
extern crate tokio_modbus;
extern crate tokio_serial;
extern crate tokio_service;

use tokio_core::reactor::Core;
use tokio_serial::{BaudRate, Serial, SerialPortSettings};
use tokio_modbus::{Client, RtuClient};
use futures::future::Future;

pub fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let tty_path = "/dev/ttyUSB0";

    let mut settings = SerialPortSettings::default();
    settings.baud_rate = BaudRate::Baud19200;
    let mut port = Serial::from_path(tty_path, &settings, &handle)
        .expect(&format!("Unable to open serial device '{}'", tty_path));
    port.set_exclusive(false)
        .expect("Unable to set serial port exlusive");

    let task = RtuClient::connect(port, 0x01, &handle).and_then(|client| {
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
