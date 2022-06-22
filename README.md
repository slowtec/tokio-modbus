# tokio-modbus

A [tokio](https://tokio.rs)-based modbus library.

[![Crates.io](https://img.shields.io/crates/v/tokio-modbus.svg)](https://crates.io/crates/tokio-modbus)
[![Docs.rs](https://docs.rs/tokio-modbus/badge.svg)](https://docs.rs/tokio-modbus/)
[![Security audit](https://github.com/slowtec/tokio-modbus/actions/workflows/security-audit.yaml/badge.svg)](https://github.com/slowtec/tokio-modbus/actions/workflows/security-audit.yaml)
[![Continuous integration](https://github.com/slowtec/tokio-modbus/actions/workflows/continuous-integration.yaml/badge.svg)](https://github.com/slowtec/tokio-modbus/actions/workflows/continuous-integration.yaml)

## Features

- Pure Rust library
- Modbus TCP or RTU at your choice
- Both `async` (non-blocking, default) and `sync` (blocking, optional)
- Client API
- Server implementations
  - for *out-of-the-box* usage or
  - as a starting point for a customized implementation
- Open source (MIT/Apache-2.0)

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

## Testing

The workspace contains documentation, tests, and examples for all available
features. Running the tests for the whole workspace only succeeds with all
features enabled:

```sh
cargo test --workspace --all-features
```

Otherwise some *doctests* that require non-default features like `sync`
are expected to fail.

## Protocol-Specification

- [MODBUS Application Protocol Specification v1.1b3 (PDF)](http://modbus.org/docs/Modbus_Application_Protocol_V1_1b3.pdf)
- [MODBUS over serial line specification and implementation guide v1.02 (PDF)](http://modbus.org/docs/Modbus_over_serial_line_V1_02.pdf)
- [MODBUS Messaging on TCP/IP Implementation Guide v1.0b (PDF)](http://modbus.org/docs/Modbus_Messaging_Implementation_Guide_V1_0b.pdf)

## License

Copyright 2017 - 2022 [slowtec GmbH](https://www.slowtec.de)

[MIT](LICENSE-MIT)/[Apache-2.0](LICENSE-APACHE)
