#[cfg(feature = "rtu")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tokio_serial::{Serial, SerialPortSettings};

    use tokio_modbus::prelude::*;

    let tty_path = "/dev/ttyUSB0";
    let slave = Slave(0x17);

    let mut settings = SerialPortSettings::default();
    settings.baud_rate = 19200;
    let port = Serial::from_path(tty_path, &settings).unwrap();

    let mut ctx = rtu::connect_slave(port, slave).await?;
    println!("Reading a sensor value");
    let rsp = ctx.read_holding_registers(0x082B, 2).await?;
    println!("Sensor value is: {:?}", rsp);

    Ok(())
}

#[cfg(not(feature = "rtu"))]
pub fn main() {
    println!("feature `rtu` is required to run this example");
    std::process::exit(1);
}
