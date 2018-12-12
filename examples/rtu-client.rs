use tokio_modbus::prelude::*;

const SLAVE_1: Slave = Slave(0x01);
const SLAVE_2: Slave = Slave(0x02);

#[cfg(feature = "rtu")]
pub fn main() {
    use futures::Future;
    use tokio_core::reactor::Core;
    use tokio_serial::{Serial, SerialPortSettings};

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let tty_path = "/dev/ttyUSB0";

    let mut settings = SerialPortSettings::default();
    settings.baud_rate = 19200;
    let port = Serial::from_path_with_handle(tty_path, &settings, &handle.new_tokio_handle())
        .expect(&format!("Unable to open serial device '{}'", tty_path));

    // On Unix you might disable the `exclusive` flag:
    // port.set_exclusive(false)
    //     .expect("Unable to set serial port exlusive");

    let task = rtu::connect(&handle, port)
        .and_then(move |mut ctx| {
            ctx.set_slave(SLAVE_1);
            println!("Reading a sensor value from {:?}", SLAVE_1);
            ctx.read_holding_registers(0x082B, 2)
                .and_then(|rsp| Ok((ctx, rsp)))
        })
        .and_then(move |(ctx, rsp)| {
            println!("Sensor value for device {:?} is: {:?}", SLAVE_1, rsp);
            Ok(ctx)
        })
        .and_then(|mut ctx| {
            ctx.set_slave(SLAVE_2);
            println!("Reading a sensor value from {:?}", SLAVE_2);
            ctx.read_holding_registers(0x082B, 2)
                .and_then(|rsp| Ok((ctx, rsp)))
        })
        .and_then(move |(_, rsp)| {
            println!("Sensor value for device {:?} is: {:?}", SLAVE_2, rsp);
            Ok(())
        });

    core.run(task).unwrap();
}

#[cfg(not(feature = "rtu"))]
pub fn main() {
    println!("feature `rtu` is required to run this example");
    std::process::exit(1);
}
