# Changelog

## v0.5.2 (2021-12-05)

- Fix (RTU): Wrong byte count offset when writing multiple coils/registers

## v0.5.1 (2021-11-21)

- Fix: require tokio/rt for sync feature
- Changed: Update methods on TCP server to be async (only concerns `tcp-server-unstable` feature)

## v0.5.0 (2021-08-20)

- Removed `sync` from default features
- Derived `Debug` for client-side RTU/TCP `Context`
- Removed client-side `SharedContext`
- Upgraded [tokio](https://tokio.rs/) version from 0.2 to 1
- Switched from deprecated [net2](https://github.com/deprecrated/net2-rs) to [socket2](https://github.com/rust-lang/socket2)

## v0.4.2 (2021-12-05)

- Fix (RTU): Wrong byte count offset when writing multiple coils/registers

## v0.4.1 (2021-08-13)

- Fixed handling of *broken pipe* errors in RTU service
- Fixed multiplication overflow for function 1 and 2 [#87](https://github.com/slowtec/tokio-modbus/pull/87)

## v0.4.0 (2020-03-13)

- New public API: moved to async/await and tokio v0.2.x
- Removed unmaintained dependency `tokio-proto`
- Make `Exception` and `ExceptionResponse` public
- Fixed `WriteSingleCoil` response to include data
- Hide server traits `Service`/`NewService` traits behind `server` feature
- Hide TCP server implementation behind `tcp-server-unstable` feature
- Improved documentation

### Breaking Changes

Due to the move to async/await and tokio v0.2.x you'll need to adjust
your current code.
Here are some lines as example:

```diff
-let mut core = Core::new().unwrap();
-let handle = core.handle();
 let socket_addr = "127.0.0.1:5502".parse().unwrap();
-let task = tcp::connect(&handle, socket_addr).and_then(move |ctx|
-    ctx.read_input_registers(0x1000, 7).and_then(move |data|
-        // ...
-    )
-);
+let mut ctx = tcp::connect(socket_addr).await?;
+let data = ctx.read_input_registers(0x1000, 7).await?;
```

## v0.3.5 (2019-09-17)

- Added missing implementation of `disconnect()` for TCP clients
- Upgraded *tokio-serial* to version 3.3

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

- Rename and relocate *Client* structs into *Context*

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
