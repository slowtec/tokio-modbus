<!-- SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# tokio-modbus

A pure [Rust](https://www.rust-lang.org)
[Modbus](https://en.wikipedia.org/wiki/Modbus) library based on
[tokio](https://tokio.rs).

[![Crates.io](https://img.shields.io/crates/v/tokio-modbus.svg)](https://crates.io/crates/tokio-modbus)
[![Docs.rs](https://docs.rs/tokio-modbus/badge.svg)](https://docs.rs/tokio-modbus/)
[![Security audit](https://github.com/slowtec/tokio-modbus/actions/workflows/security-audit.yaml/badge.svg)](https://github.com/slowtec/tokio-modbus/actions/workflows/security-audit.yaml)
[![Continuous integration](https://github.com/slowtec/tokio-modbus/actions/workflows/continuous-integration.yaml/badge.svg)](https://github.com/slowtec/tokio-modbus/actions/workflows/continuous-integration.yaml)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![Apache 2.0 licensed](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](./LICENSE-APACHE)

Modbus is based on a
[_master/slave_](<https://en.wikipedia.org/wiki/Master/slave_(technology)>)
communication pattern. To avoid confusion with the Tokio terminology the
_master_ is called _client_ and the _slave_ is called _server_ in this library.

## Features

- Pure Rust library
- Modbus TCP or RTU at your choice
- Both `async` (non-blocking, default) and `sync` (blocking, optional)
- Client API
- Server implementations
  - for _out-of-the-box_ usage or
  - as a starting point for a customized implementation
- Open source (MIT/Apache-2.0)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
tokio-modbus = "*"
```

### Cargo Features

- `"rtu"`: Asynchronous RTU client (default)
- `"tcp"`: Asynchronous TCP client (default)
- `"rtu-sync`: Synchronous RTU client
- `"tcp-sync"`: Synchronous TCP client
- `"rtu-server"`: (Asynchronous) RTU server
- `"tcp-server"`: (Asynchronous) TCP server

#### Examples

If you only need an asynchronous TCP client add the following line to your
Cargo.toml file:

```toml
[dependencies]
tokio-modbus = { version = "*", default-features = false, features = ["tcp"] }
```

For an asynchronous RTU client:

```toml
[dependencies]
tokio-modbus = { version = "*", default-features = false, features = ["rtu"] }
```

For an RTU server:

```toml
[dependencies]
tokio-modbus = { version = "*", default-features = false, features = ["rtu-server"] }
```

For a TCP server:

```toml
[dependencies]
tokio-modbus = { version = "*", default-features = false, features = ["tcp-server"] }
```

## Examples

Various examples for Modbus RTU and TCP using either the asynchronous or
synchronous API can be found in the
[examples](https://github.com/slowtec/tokio-modbus/tree/main/examples) folder.

## Testing

The workspace contains documentation, tests, and examples for all available
features.

```sh
cargo test --workspace
cargo test --workspace --all-features
```

## Protocol-Specification

- [Modbus Application Protocol Specification v1.1b3 (PDF)](http://modbus.org/docs/Modbus_Application_Protocol_V1_1b3.pdf)
- [Modbus over serial line specification and implementation guide v1.02 (PDF)](http://modbus.org/docs/Modbus_over_serial_line_V1_02.pdf)
- [Modbus Messaging on TCP/IP Implementation Guide v1.0b (PDF)](http://modbus.org/docs/Modbus_Messaging_Implementation_Guide_V1_0b.pdf)

## License

Copyright (c) 2017-2023 [slowtec GmbH](https://www.slowtec.de)

[MIT](LICENSES/MIT.txt)/[Apache-2.0](LICENSES/Apache-2.0.txt)
