# Changelog

## v0.3.0 (yyyy-mm-dd)

- New public API

### Breaking Changes

- Make all public crate exports accessible in the new `prelude` module

  ```diff
  -  use tokio_modbus::*;
  +  use tokio_modbus::prelude::*;
  ```

- Rename and relocate client traits

  ```diff
  -  client::ModbusClient
  +  client::Client
  ```

  ```diff
  -  client::SyncModbusClient
  +  client::sync::Client
  ```

- Rename and relocate client structs

  ```diff
  -  client::Client
  +  client::Connection
  ```

  ```diff
  -  client::SyncClient
  +  client::sync::Connection
  ```

- Split `Client` trait into `Client` + `Reader` + `Writer` traits

- Use free functions in (nested) submodules for establishing client connections

  ```diff
  -  Client::connect_tcp(...);
  +  tcp::connect(...);
  ```

  ```diff
  -  Client::connect_rtu(...);
  +  rtu::connect(...);
  ```

  ```diff
  -  SyncClient::connect_tcp(...);
  +  sync::tcp::connect(...);
  ```

  ```diff
  -  SyncClient::connect_rtu(...);
  +  sync::rtu::connect(...);
  ```

- Move TCP server into submodule

  ```diff
  -  Server::new_tcp(socket_addr).serve(...);
  +  tcp::Server::new(socket_addr).serve(...);
  ```

## v0.2.3 (2018-12-03)

- fix decoding of incomplete RTU frames

## v0.2.2 (2018-12-01)

- fix compilation with `features = ["rtu"]`
- refactor: use `tokio-codec`
- refactor: use `put_u16_be` instead of `put_u16::<BigEndian>`
- refactor: prepare for compilation with `edition = "2018"`

## v0.2.1 (2018-04-04)

- fix codec: create buffers with correct capacity

## v0.2.0 (2018-03-30)

- add simple tcp server implementation
- add sync clients
- use tokio-serial v0.6.x

### Breaking Changes

- Changed Client API
  ```diff
  -  use tokio_modbus::{Client, TcpClient};
  +  use tokio_modbus::*;
  ```
  ```diff
  -  RtuClient::connect(port, server_addr, &handle)
  +  Client::connect_rtu(port, server_addr, &handle)
  ```
  ```diff
  -  TcpClient::connect(&addr, &handle)
  +  Client::connect_tcp(&addr, &handle)
  ```

## v0.1.0 (2018-01-21)

- initial implementation
