# tokio-modbus

A [tokio](https://tokio.rs)-based modbus library.

[![Crates.io version](https://img.shields.io/crates/v/tokio-modbus.svg)](https://crates.io/crates/tokio-modbus)
[![Docs](https://docs.rs/tokio-modbus/badge.svg)](https://docs.rs/tokio-modbus/)
[![Build Status](https://travis-ci.org/slowtec/tokio-modbus.svg?branch=master)](https://travis-ci.org/slowtec/tokio-modbus)
[![Coverage Status](https://coveralls.io/repos/github/slowtec/tokio-modbus/badge.svg?branch=master)](https://coveralls.io/github/slowtec/tokio-modbus?branch=master)

## Features

- pure Rust library
- async (non-blocking)
- sync (blocking)
- Modbus TCP
- Modbus RTU
- Client & Server
- Open Source (MIT/Apache-2.0)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
tokio-modbus = "*"
```

If you like to use Modbus TCP only:

```toml
[dependencies]
tokio-modbus = { version = "*", default-features = false, features = ["tcp"] }
```

If you like to use Modbus RTU only:

```toml
[dependencies]
tokio-modbus = { version = "*", default-features = false, features = ["rtu"] }
```

## Examples

### TCP client

```rust
extern crate futures;
extern crate tokio_core;
extern crate tokio_modbus;

use tokio_core::reactor::Core;
use futures::future::Future;
use tokio_modbus::prelude::*;

pub fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let addr = "192.168.0.222:502".parse().unwrap();

    let task = TcpClient::connect(&addr, &handle).and_then(|client| {
        client
            .read_input_registers(0x1000, 7)
            .and_then(move |data| {
                println!("Response is '{:?}'", data);
                Ok(())
            })
    });
    core.run(task).unwrap();
}
```

### Sync TCP client

```rust
extern crate tokio_modbus;

use tokio_modbus::prelude::*;

pub fn main() {
    let socket_addr = "192.168.0.222:502".parse().unwrap();
    let client = SyncClient::connect_tcp(&socket_addr).unwrap();
    let buff = client.read_input_registers(0x1000, 7).unwrap();
    println!("Response is '{:?}'", buff);
}
```

### RTU client

```rust
extern crate futures;
extern crate tokio_core;
extern crate tokio_modbus;
extern crate tokio_service;

extern crate tokio_serial;

pub fn main() {
    use futures::future::Future;
    use tokio_core::reactor::Core;
    use tokio_modbus::prelude::*;
    use tokio_serial::{Serial, SerialPortSettings};

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let tty_path = "/dev/ttyUSB0";
    let server_addr = 0x01;

    let mut settings = SerialPortSettings::default();
    settings.baud_rate = 19200;
    let mut port = Serial::from_path_with_handle(tty_path, &settings, &handle.new_tokio_handle())
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
```

More examples can be found in the [examples](https://github.com/slowtec/tokio-modbus/tree/master/examples) folder.

## Protocol-Specification

- [MODBUS Application Protocol Specification v1.1b3 (PDF)](http://modbus.org/docs/Modbus_Application_Protocol_V1_1b3.pdf)
- [MODBUS over serial line specification and implementation guide v1.02 (PDF)](http://modbus.org/docs/Modbus_over_serial_line_V1_02.pdf)
- [MODBUS Messaging on TCP/IP Implementation Guide v1.0b (PDF)](http://modbus.org/docs/Modbus_Messaging_Implementation_Guide_V1_0b.pdf)

## License

Copyright 2018 [slowtec GmbH](https://www.slowtec.de)

MIT/Apache-2.0
