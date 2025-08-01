// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus clients

use std::{borrow::Cow, fmt::Debug, io};

use async_trait::async_trait;

use crate::{frame::*, slave::*, Result};

#[cfg(feature = "rtu")]
pub mod rtu;

#[cfg(feature = "tcp")]
pub mod tcp;

#[cfg(feature = "sync")]
pub mod sync;

/// Transport independent asynchronous client trait
#[async_trait]
pub trait Client: SlaveContext + Send + Debug {
    /// Invokes a _Modbus_ function.
    async fn call(&mut self, request: Request<'_>) -> Result<Response>;

    /// Disconnects the client.
    ///
    /// Permanently disconnects the client by shutting down the
    /// underlying stream in a graceful manner (`AsyncDrop`).
    ///
    /// Dropping the client without explicitly disconnecting it
    /// beforehand should also work and free all resources. The
    /// actual behavior might depend on the underlying transport
    /// protocol (RTU/TCP) that is used by the client.
    async fn disconnect(&mut self) -> io::Result<()>;
}

/// Asynchronous _Modbus_ reader
#[async_trait]
pub trait Reader: Client {
    /// Read multiple coils (0x01)
    async fn read_coils(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Coil>>;

    /// Read multiple discrete inputs (0x02)
    async fn read_discrete_inputs(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Coil>>;

    /// Read multiple holding registers (0x03)
    async fn read_holding_registers(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Word>>;

    /// Read multiple input registers (0x04)
    async fn read_input_registers(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Word>>;

    /// Read and write multiple holding registers (0x17)
    ///
    /// The write operation is performed before the read unlike
    /// the name of the operation might suggest!
    async fn read_write_multiple_registers(
        &mut self,
        read_addr: Address,
        read_count: Quantity,
        write_addr: Address,
        write_data: &[Word],
    ) -> Result<Vec<Word>>;

    /// Read device identification (0x2B / 0x0E)
    async fn read_device_identification(
        &mut self,
        read_code: ReadCode,
        object_id: ObjectId,
    ) -> Result<(ConformityLevel, MoreFollows, NextObjectId, DeviceIdObjects)>;
}

/// Asynchronous Modbus writer
#[async_trait]
pub trait Writer: Client {
    /// Write a single coil (0x05)
    async fn write_single_coil(&mut self, addr: Address, coil: Coil) -> Result<()>;

    /// Write a single holding register (0x06)
    async fn write_single_register(&mut self, addr: Address, word: Word) -> Result<()>;

    /// Write multiple coils (0x0F)
    async fn write_multiple_coils(&mut self, addr: Address, coils: &'_ [Coil]) -> Result<()>;

    /// Write multiple holding registers (0x10)
    async fn write_multiple_registers(&mut self, addr: Address, words: &[Word]) -> Result<()>;

    /// Set or clear individual bits of a holding register (0x16)
    async fn masked_write_register(
        &mut self,
        addr: Address,
        and_mask: Word,
        or_mask: Word,
    ) -> Result<()>;
}

/// Asynchronous Modbus client context
#[derive(Debug)]
pub struct Context {
    client: Box<dyn Client>,
}

impl From<Box<dyn Client>> for Context {
    fn from(client: Box<dyn Client>) -> Self {
        Self { client }
    }
}

impl From<Context> for Box<dyn Client> {
    fn from(val: Context) -> Self {
        val.client
    }
}

#[async_trait]
impl Client for Context {
    async fn call(&mut self, request: Request<'_>) -> Result<Response> {
        self.client.call(request).await
    }

    async fn disconnect(&mut self) -> io::Result<()> {
        self.client.disconnect().await
    }
}

impl SlaveContext for Context {
    fn set_slave(&mut self, slave: Slave) {
        self.client.set_slave(slave);
    }
}

#[async_trait]
impl Reader for Context {
    async fn read_coils<'a>(&'a mut self, addr: Address, cnt: Quantity) -> Result<Vec<Coil>> {
        self.client
            .call(Request::ReadCoils(addr, cnt))
            .await
            .map(|result| {
                result.map(|response| match response {
                    Response::ReadCoils(mut coils) => {
                        debug_assert!(coils.len() >= cnt.into());
                        coils.truncate(cnt.into());
                        coils
                    }
                    _ => unreachable!("call() should reject mismatching responses"),
                })
            })
    }

    async fn read_discrete_inputs<'a>(
        &'a mut self,
        addr: Address,
        cnt: Quantity,
    ) -> Result<Vec<Coil>> {
        self.client
            .call(Request::ReadDiscreteInputs(addr, cnt))
            .await
            .map(|result| {
                result.map(|response| match response {
                    Response::ReadDiscreteInputs(mut coils) => {
                        debug_assert!(coils.len() >= cnt.into());
                        coils.truncate(cnt.into());
                        coils
                    }
                    _ => unreachable!("call() should reject mismatching responses"),
                })
            })
    }

    async fn read_input_registers<'a>(
        &'a mut self,
        addr: Address,
        cnt: Quantity,
    ) -> Result<Vec<Word>> {
        self.client
            .call(Request::ReadInputRegisters(addr, cnt))
            .await
            .map(|result| {
                result.map(|response| match response {
                    Response::ReadInputRegisters(words) => {
                        debug_assert_eq!(words.len(), cnt.into());
                        words
                    }
                    _ => unreachable!("call() should reject mismatching responses"),
                })
            })
    }

    async fn read_holding_registers<'a>(
        &'a mut self,
        addr: Address,
        cnt: Quantity,
    ) -> Result<Vec<Word>> {
        self.client
            .call(Request::ReadHoldingRegisters(addr, cnt))
            .await
            .map(|result| {
                result.map(|response| match response {
                    Response::ReadHoldingRegisters(words) => {
                        debug_assert_eq!(words.len(), cnt.into());
                        words
                    }
                    _ => unreachable!("call() should reject mismatching responses"),
                })
            })
    }

    async fn read_write_multiple_registers<'a>(
        &'a mut self,
        read_addr: Address,
        read_count: Quantity,
        write_addr: Address,
        write_data: &[Word],
    ) -> Result<Vec<Word>> {
        self.client
            .call(Request::ReadWriteMultipleRegisters(
                read_addr,
                read_count,
                write_addr,
                Cow::Borrowed(write_data),
            ))
            .await
            .map(|result| {
                result.map(|response| match response {
                    Response::ReadWriteMultipleRegisters(words) => {
                        debug_assert_eq!(words.len(), read_count.into());
                        words
                    }
                    _ => unreachable!("call() should reject mismatching responses"),
                })
            })
    }

    async fn read_device_identification(
        &mut self,
        read_code: ReadCode,
        object_id: ObjectId,
    ) -> Result<(ConformityLevel, MoreFollows, NextObjectId, DeviceIdObjects)> {
        self.client
            .call(Request::ReadDeviceIdentification(read_code, object_id))
            .await
            .map(|result| {
                result.map(|response| match response {
                    Response::ReadDeviceIdentification(
                        _read_code,
                        conformity_level,
                        more_follows,
                        next_object_id,
                        device_id_objects,
                    ) => (
                        conformity_level,
                        more_follows,
                        next_object_id,
                        device_id_objects,
                    ),
                    _ => unreachable!("call() should reject mismatching responses"),
                })
            })
    }
}

#[async_trait]
impl Writer for Context {
    async fn write_single_coil<'a>(&'a mut self, addr: Address, coil: Coil) -> Result<()> {
        self.client
            .call(Request::WriteSingleCoil(addr, coil))
            .await
            .map(|result| {
                result.map(|response| match response {
                    Response::WriteSingleCoil(rsp_addr, rsp_coil) => {
                        debug_assert_eq!(addr, rsp_addr);
                        debug_assert_eq!(coil, rsp_coil);
                    }
                    _ => unreachable!("call() should reject mismatching responses"),
                })
            })
    }

    async fn write_multiple_coils<'a>(&'a mut self, addr: Address, coils: &[Coil]) -> Result<()> {
        let cnt = coils.len();
        self.client
            .call(Request::WriteMultipleCoils(addr, Cow::Borrowed(coils)))
            .await
            .map(|result| {
                result.map(|response| match response {
                    Response::WriteMultipleCoils(rsp_addr, rsp_cnt) => {
                        debug_assert_eq!(addr, rsp_addr);
                        debug_assert_eq!(cnt, rsp_cnt.into());
                    }
                    _ => unreachable!("call() should reject mismatching responses"),
                })
            })
    }

    async fn write_single_register<'a>(&'a mut self, addr: Address, word: Word) -> Result<()> {
        self.client
            .call(Request::WriteSingleRegister(addr, word))
            .await
            .map(|result| {
                result.map(|response| match response {
                    Response::WriteSingleRegister(rsp_addr, rsp_word) => {
                        debug_assert_eq!(addr, rsp_addr);
                        debug_assert_eq!(word, rsp_word);
                    }
                    _ => unreachable!("call() should reject mismatching responses"),
                })
            })
    }

    async fn write_multiple_registers<'a>(
        &'a mut self,
        addr: Address,
        data: &[Word],
    ) -> Result<()> {
        let cnt = data.len();
        self.client
            .call(Request::WriteMultipleRegisters(addr, Cow::Borrowed(data)))
            .await
            .map(|result| {
                result.map(|response| match response {
                    Response::WriteMultipleRegisters(rsp_addr, rsp_cnt) => {
                        debug_assert_eq!(addr, rsp_addr);
                        debug_assert_eq!(cnt, rsp_cnt.into());
                    }
                    _ => unreachable!("call() should reject mismatching responses"),
                })
            })
    }

    async fn masked_write_register<'a>(
        &'a mut self,
        addr: Address,
        and_mask: Word,
        or_mask: Word,
    ) -> Result<()> {
        self.client
            .call(Request::MaskWriteRegister(addr, and_mask, or_mask))
            .await
            .map(|result| {
                result.map(|response| match response {
                    Response::MaskWriteRegister(rsp_addr, rsp_and_mask, rsp_or_mask) => {
                        debug_assert_eq!(addr, rsp_addr);
                        debug_assert_eq!(and_mask, rsp_and_mask);
                        debug_assert_eq!(or_mask, rsp_or_mask);
                    }
                    _ => unreachable!("call() should reject mismatching responses"),
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, Result};

    use super::*;
    use std::{io, sync::Mutex};

    #[derive(Default, Debug)]
    pub(crate) struct ClientMock {
        slave: Option<Slave>,
        last_request: Mutex<Option<Request<'static>>>,
        next_response: Option<Result<Response>>,
    }

    #[allow(dead_code)]
    impl ClientMock {
        pub(crate) fn slave(&self) -> Option<Slave> {
            self.slave
        }

        pub(crate) fn last_request(&self) -> &Mutex<Option<Request<'static>>> {
            &self.last_request
        }

        pub(crate) fn set_next_response(&mut self, next_response: Result<Response>) {
            self.next_response = Some(next_response);
        }
    }

    #[async_trait]
    impl Client for ClientMock {
        async fn call(&mut self, request: Request<'_>) -> Result<Response> {
            *self.last_request.lock().unwrap() = Some(request.into_owned());
            match self.next_response.take().unwrap() {
                Ok(response) => Ok(response),
                Err(Error::Transport(err)) => {
                    Err(io::Error::new(err.kind(), format!("{err}")).into())
                }
                Err(err) => Err(err),
            }
        }

        async fn disconnect(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl SlaveContext for ClientMock {
        fn set_slave(&mut self, slave: Slave) {
            self.slave = Some(slave);
        }
    }

    #[test]
    fn read_some_coils() {
        // The protocol will always return entire bytes with, i.e.
        // a multiple of 8 coils.
        let response_coils = [true, false, false, true, false, true, false, true];
        for num_coils in 1..8 {
            let mut client = Box::<ClientMock>::default();
            client.set_next_response(Ok(Ok(Response::ReadCoils(response_coils.to_vec()))));
            let mut context = Context { client };
            context.set_slave(Slave(1));
            let coils = futures::executor::block_on(context.read_coils(1, num_coils))
                .unwrap()
                .unwrap();
            assert_eq!(&response_coils[0..num_coils as usize], &coils[..]);
        }
    }

    #[test]
    fn read_some_discrete_inputs() {
        // The protocol will always return entire bytes with, i.e.
        // a multiple of 8 coils.
        let response_inputs = [true, false, false, true, false, true, false, true];
        for num_inputs in 1..8 {
            let mut client = Box::<ClientMock>::default();
            client.set_next_response(Ok(Ok(Response::ReadDiscreteInputs(
                response_inputs.to_vec(),
            ))));
            let mut context = Context { client };
            context.set_slave(Slave(1));
            let inputs = futures::executor::block_on(context.read_discrete_inputs(1, num_inputs))
                .unwrap()
                .unwrap();
            assert_eq!(&response_inputs[0..num_inputs as usize], &inputs[..]);
        }
    }
}
