#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use serial_io::{build, AsyncSerial};

    use tokio_modbus::prelude::*;

    let tty_path = "/dev/ttyUSB0";
    let slave = Slave(0x17);

    let builder = build(tty_path, 19200);
    let port = AsyncSerial::from_builder(&builder).unwrap();

    let mut ctx = rtu::connect_slave(port, slave).await?;
    println!("Reading a sensor value");
    let rsp = ctx.read_holding_registers(0x082B, 2).await?;
    println!("Sensor value is: {:?}", rsp);

    Ok(())
}
