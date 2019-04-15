use futures::{future, Future};

use std::{cell::RefCell, io::Error, rc::Rc};

use tokio_core::reactor::{Core, Handle};

use tokio_modbus::prelude::{
    client::util::{reconnect_shared_context, NewContext, SharedContext},
    *,
};

const SLAVE_1: Slave = Slave(0x01);
const SLAVE_2: Slave = Slave(0x02);

#[cfg(feature = "rtu")]
pub fn main() {
    use tokio_serial::{Serial, SerialPortSettings};

    let mut core = Core::new().unwrap();

    #[derive(Debug)]
    struct SerialConfig {
        path: String,
        settings: SerialPortSettings,
        handle: Handle,
    }

    impl NewContext for SerialConfig {
        fn new_context(&self) -> Box<dyn Future<Item = client::Context, Error = Error>> {
            let handle = self.handle.clone();
            Box::new(
                future::result(Serial::from_path_with_handle(
                    &self.path,
                    &self.settings,
                    &self.handle.new_tokio_handle(),
                ))
                .and_then(move |port| rtu::connect(&handle, port)),
            )
        }
    }

    let serial_config = SerialConfig {
        path: "/dev/ttyUSB0".into(),
        settings: SerialPortSettings {
            baud_rate: 19200,
            ..Default::default()
        },
        handle: core.handle(),
    };
    println!("Configuration: {:?}", serial_config);

    // A shared, reconnectable context is not actually needed in this
    // simple example. Nevertheless we use it here to demonstrate how
    // it works.
    let shared_context = Rc::new(RefCell::new(SharedContext::new(
        None, // no initial context, i.e. not connected
        Box::new(serial_config),
    )));

    // Reconnect for connecting an initial context
    let task = reconnect_shared_context(&shared_context)
        .map(move |()| {
            assert!(shared_context.borrow().is_connected());
            println!("Connected");
            shared_context
        })
        .and_then(move |shared_context| {
            println!("Reading a sensor value from {:?}", SLAVE_1);
            let context = shared_context.borrow().share_context().unwrap();
            let mut context = context.borrow_mut();
            context.set_slave(SLAVE_1);
            context
                .read_holding_registers(0x082B, 2)
                .map(move |response| {
                    println!("Sensor value for device {:?} is: {:?}", SLAVE_1, response);
                    shared_context // Continue
                })
        })
        .and_then(move |shared_context| {
            println!("Reading a sensor value from {:?}", SLAVE_2);
            let context = shared_context.borrow().share_context().unwrap();
            let mut context = context.borrow_mut();
            context.set_slave(SLAVE_2);
            context
                .read_holding_registers(0x082B, 2)
                .map(move |response| {
                    println!("Sensor value for device {:?} is: {:?}", SLAVE_2, response);
                    // Done
                })
        });

    core.run(task).unwrap();
}

#[cfg(not(feature = "rtu"))]
pub fn main() {
    println!("feature `rtu` is required to run this example");
    std::process::exit(1);
}
