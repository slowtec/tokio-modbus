# tokio-modbus

A [tokio](https://tokio.rs)-based modbus library.

[![Crates.io](https://img.shields.io/crates/v/tokio-modbus.svg)](https://crates.io/crates/tokio-modbus)
[![Docs.rs](https://docs.rs/tokio-modbus/badge.svg)](https://docs.rs/tokio-modbus/)
[![Security audit](https://github.com/github/docs/actions/workflows/security_audit.yaml/badge.svg?branch=master)](https://github.com/slowtec/tokio-modbus/actions/workflows/security_audit.yaml)
[![Continuous integration](https://github.com/github/docs/actions/workflows/continuous_integration.yaml/badge.svg?branch=master)](https://github.com/slowtec/tokio-modbus/actions/workflows/continuous_integration.yaml)
[![Test coverage](https://coveralls.io/repos/github/slowtec/tokio-modbus/badge.svg?branch=master)](https://coveralls.io/github/slowtec/tokio-modbus?branch=master)

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

If you like to build a TCP server:

```toml
[dependencies]
tokio-modbus = { version = "*", default-features = false, features = ["tcp", "server"] }
```

## Examples

Various examples for Modbus RTU and TCP using either the asynchronous
or synchronous API can be found in the
[examples](https://github.com/slowtec/tokio-modbus/tree/master/examples)
folder.

## Protocol-Specification

- [MODBUS Application Protocol Specification v1.1b3 (PDF)](http://modbus.org/docs/Modbus_Application_Protocol_V1_1b3.pdf)
- [MODBUS over serial line specification and implementation guide v1.02 (PDF)](http://modbus.org/docs/Modbus_over_serial_line_V1_02.pdf)
- [MODBUS Messaging on TCP/IP Implementation Guide v1.0b (PDF)](http://modbus.org/docs/Modbus_Messaging_Implementation_Guide_V1_0b.pdf)

## License

Copyright 2017 - 2020 [slowtec GmbH](https://www.slowtec.de)

MIT/Apache-2.0
