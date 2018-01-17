extern crate futures;
extern crate tokio_core;
extern crate tokio_modbus;
extern crate tokio_serial;
extern crate tokio_service;

use tokio_core::reactor::Core;
use tokio_serial::{Serial, SerialPortSettings, BaudRate};
use tokio_modbus::{RtuClient,Request, Response};
use futures::future::Future;
use tokio_service::Service;

pub fn main() {

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let tty_path = "/dev/ttyUSB0";

    let mut settings = SerialPortSettings::default();
    settings.baud_rate = BaudRate::Baud19200;
    let mut port = Serial::from_path(tty_path, &settings, &handle)
        .expect(&format!("Unable to open serial device '{}'", tty_path));
    port.set_exclusive(false).expect("Unable to set serial port exlusive");

    let task = RtuClient::connect(port, 0x01, &handle).and_then(|client| {
        println!("Reading a sensor value");
        client
            .call(Request::ReadHoldingRegisters(0x082B, 2))
            .and_then(move |response| {
                match response {
                    Response::ReadHoldingRegisters(res) => {
                        println!("Sensor value is: {:?}",res);
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
