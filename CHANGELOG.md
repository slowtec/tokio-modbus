# Changelog

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
