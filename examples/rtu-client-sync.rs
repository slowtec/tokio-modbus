pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    use serial_io::build;

    use tokio_modbus::prelude::*;

    let tty_path = "/dev/ttyUSB0";
    let slave = Slave(0x17);

    let builder = build(tty_path, 19200);

    let mut ctx = sync::rtu::connect_slave(&builder, slave)?;
    println!("Reading a sensor value");
    let rsp = ctx.read_holding_registers(0x082B, 2)?;
    println!("Sensor value is: {:?}", rsp);

    Ok(())
}
