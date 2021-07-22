#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{cell::RefCell, future::Future, io::Error, pin::Pin, rc::Rc};

    use tokio_modbus::client::{
        rtu,
        util::{reconnect_shared_context, NewContext, SharedContext},
        Context,
    };
    use tokio_modbus::prelude::*;
    use tokio_serial::{SerialPortBuilder, SerialStream};

    const SLAVE_1: Slave = Slave(0x01);
    const SLAVE_2: Slave = Slave(0x02);

    #[derive(Debug)]
    struct SerialConfig {
        builder: SerialPortBuilder,
    }

    impl NewContext for SerialConfig {
        fn new_context(&self) -> Pin<Box<dyn Future<Output = Result<Context, Error>>>> {
            let serial = SerialStream::open(&self.builder);
            Box::pin(async {
                let port = serial?;
                rtu::connect(port).await
            })
        }
    }

    let serial_config = SerialConfig {
        builder: tokio_serial::new("/dev/ttyUSB0", 19200),
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
    reconnect_shared_context(&shared_context).await?;

    assert!(shared_context.borrow().is_connected());
    println!("Connected");

    println!("Reading a sensor value from {:?}", SLAVE_1);
    let context = shared_context.borrow().share_context().unwrap();
    let mut context = context.borrow_mut();
    context.set_slave(SLAVE_1);
    let response = context.read_holding_registers(0x082B, 2).await?;
    println!("Sensor value for device {:?} is: {:?}", SLAVE_1, response);

    println!("Reading a sensor value from {:?}", SLAVE_2);
    context.set_slave(SLAVE_2);
    let response = context.read_holding_registers(0x082B, 2).await?;

    println!("Sensor value for device {:?} is: {:?}", SLAVE_2, response);

    Ok(())
}
