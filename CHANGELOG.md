# Changelog

## v0.3.4 (2019-05-21)

- Disabled the default features of *tokio-serial* to exclude an unused
  dependency on *libudev* inherited from *mio-serial*

## v0.3.3 (2019-05-16)

- Fixed reading coils: Truncate response payload to match the requested
  number of coils or discrete inputs.

## v0.3.2 (2019-04-15)

- Client: Added a `Disconnect` request as *poison pill* for stopping
  the client service and to release the underlying transport
- Added utilities to share a single Modbus context within a thread for
  communicating with multiple devices
- Added utility functions to disconnect and reconnect stale connections
  after errors
- Minimal Rust version: `1.34.0`

### Potential breaking change

Extending the `Request` enum with the new variant `Disconnect` might break
existing code. This variant is only used internally within the client and
will never be sent across the wire and can safely be ignored by both clients
and servers!

## v0.3.1 (2019-04-08)

- RTU client: Use a generic async transport instead of `Serial`

## v0.3.0 (2019-04-02)

- New public API
- Client: Change devices while connected
- TCP Client: Connect to RTU devices via gateway (unit identifier)
- RTU Client: Try to recover from frame errors

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

- Rename and relocate _Client_ structs into _Context_

  ```diff
  -  client::Client
  +  client::Context
  ```

  ```diff
  -  client::SyncClient
  +  client::sync::Context
  ```

- Split `Client` trait into `Client` + `Reader` + `Writer` traits

- Use free functions in (nested) submodules for creating/connecting a client context

  ```diff
  -  Client::connect_tcp(...);
  +  tcp::connect(...) or tcp::connect_device(...);
  ```

  ```diff
  -  Client::connect_rtu(...);
  +  rtu::connect_device(...);
  ```

  ```diff
  -  SyncClient::connect_tcp(...);
  +  sync::tcp::connect(...) or sync::tcp::connect_device(...);
  ```

  ```diff
  -  SyncClient::connect_rtu(...);
  +  sync::rtu::connect_device(...);
  ```

- Reorder parameters of asynchronous connect() functions,
  i.e. the Tokio handle is always the first parameter

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
