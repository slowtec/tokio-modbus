// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus clients

use std::{borrow::Cow, fmt::Debug, io};

use async_trait::async_trait;

use crate::{error::unexpected_rsp_code_panic_msg, frame::*, slave::*, Result};

#[cfg(feature = "rtu")]
pub mod rtu;

#[cfg(feature = "tcp")]
pub mod tcp;

#[cfg(feature = "sync")]
pub mod sync;

/// Transport independent asynchronous client trait
#[async_trait]
pub trait Client: SlaveContext + Send + Debug {
    /// Invoke a Modbus function
    async fn call(&mut self, request: Request<'_>) -> Result<Response>;
}

/// Asynchronous Modbus reader
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

impl Context {
    /// Disconnect the client
    pub async fn disconnect(&mut self) -> Result<()> {
        // Disconnecting is expected to fail!
        let res = self.client.call(Request::Disconnect).await;
        match res {
            Ok(_) => unreachable!(),
            Err(err) => match err.kind() {
                io::ErrorKind::NotConnected | io::ErrorKind::BrokenPipe => Ok(Ok(())),
                _ => Err(err),
            },
        }
    }
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
            .map(|modbus_rsp| {
                modbus_rsp.map(|rsp| match rsp {
                    Response::ReadCoils(mut coils) => {
                        debug_assert!(coils.len() >= cnt.into());
                        coils.truncate(cnt.into());
                        coils
                    }
                    others => {
                        // NOTE: A call to `Client::call` implementation *MUST* always return the `Response` variant matching the `Request` one.
                        // TIPS: This can be ensured via a call to `verify_response_header`( in 'src/service/mod.rs') before returning from `Client::call`.
                        unreachable!(
                            "{}",
                            unexpected_rsp_code_panic_msg(
                                FunctionCode::ReadCoils,
                                others.function_code()
                            ),
                        )
                    }
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
            .map(|modbus_rsp| {
                modbus_rsp.map(|rsp| match rsp {
                    Response::ReadDiscreteInputs(mut coils) => {
                        debug_assert!(coils.len() >= cnt.into());
                        coils.truncate(cnt.into());
                        coils
                    }
                    others => {
                        // NOTE: A call to `Client::call` implementation *MUST* always return the `Response` variant matching the `Request` one.
                        // TIPS: This can be ensured via a call to `verify_response_header`( in 'src/service/mod.rs') before returning from `Client::call`.
                        unreachable!(
                            "{}",
                            unexpected_rsp_code_panic_msg(
                                FunctionCode::ReadDiscreteInputs,
                                others.function_code()
                            ),
                        )
                    }
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
            .map(|modbus_rsp| {
                modbus_rsp.map(|rsp| match rsp {
                    Response::ReadInputRegisters(words) => {
                        debug_assert_eq!(words.len(), cnt.into());
                        words
                    }
                    others => {
                        // NOTE: A call to `Client::call` implementation *MUST* always return the `Response` variant matching the `Request` one.
                        // TIPS: This can be ensured via a call to `verify_response_header`( in 'src/service/mod.rs') before returning from `Client::call`.
                        unreachable!(
                            "{}",
                            unexpected_rsp_code_panic_msg(
                                FunctionCode::ReadInputRegisters,
                                others.function_code()
                            ),
                        )
                    }
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
            .map(|modbus_rsp| {
                modbus_rsp.map(|rsp| match rsp {
                    Response::ReadHoldingRegisters(words) => {
                        debug_assert_eq!(words.len(), cnt.into());
                        words
                    }
                    others => {
                        // NOTE: A call to `Client::call` implementation *MUST* always return the `Response` variant matching the `Request` one.
                        // TIPS: This can be ensured via a call to `verify_response_header`( in 'src/service/mod.rs') before returning from `Client::call`.
                        unreachable!(
                            "{}",
                            unexpected_rsp_code_panic_msg(
                                FunctionCode::ReadHoldingRegisters,
                                others.function_code()
                            ),
                        )
                    }
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
            .map(|modbus_rsp| {
                modbus_rsp.map(|rsp| match rsp {
                    Response::ReadWriteMultipleRegisters(words) => {
                        debug_assert_eq!(words.len(), read_count.into());
                        words
                    }
                    others => {
                        // NOTE: A call to `Client::call` implementation *MUST* always return the `Response` variant matching the `Request` one.
                        // TIPS: This can be ensured via a call to `verify_response_header`( in 'src/service/mod.rs') before returning from `Client::call`.
                        unreachable!(
                            "{}",
                            unexpected_rsp_code_panic_msg(
                                FunctionCode::ReadWriteMultipleRegisters,
                                others.function_code()
                            ),
                        )
                    }
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
            .map(|modbus_rsp| {
                modbus_rsp.map(|rsp| match rsp {
                    Response::WriteSingleCoil(rsp_addr, rsp_coil) => {
                        debug_assert_eq!(addr, rsp_addr);
                        debug_assert_eq!(coil, rsp_coil);
                    }
                    others => {
                        // NOTE: A call to `Client::call` implementation *MUST* always return the `Response` variant matching the `Request` one.
                        // TIPS: This can be ensured via a call to `verify_response_header`( in 'src/service/mod.rs') before returning from `Client::call`.
                        unreachable!(
                            "{}",
                            unexpected_rsp_code_panic_msg(
                                FunctionCode::WriteSingleCoil,
                                others.function_code()
                            ),
                        )
                    }
                })
            })
    }

    async fn write_multiple_coils<'a>(&'a mut self, addr: Address, coils: &[Coil]) -> Result<()> {
        let cnt = coils.len();

        self.client
            .call(Request::WriteMultipleCoils(addr, Cow::Borrowed(coils)))
            .await
            .map(|modbus_rsp| {
                modbus_rsp.map(|rsp| match rsp {
                    Response::WriteMultipleCoils(rsp_addr, rsp_cnt) => {
                        debug_assert_eq!(addr, rsp_addr);
                        debug_assert_eq!(cnt, rsp_cnt.into());
                    }
                    others => {
                        // NOTE: A call to `Client::call` implementation *MUST* always return the `Response` variant matching the `Request` one.
                        // TIPS: This can be ensured via a call to `verify_response_header`( in 'src/service/mod.rs') before returning from `Client::call`.
                        unreachable!(
                            "{}",
                            unexpected_rsp_code_panic_msg(
                                FunctionCode::WriteMultipleCoils,
                                others.function_code()
                            ),
                        )
                    }
                })
            })
    }

    async fn write_single_register<'a>(&'a mut self, addr: Address, word: Word) -> Result<()> {
        self.client
            .call(Request::WriteSingleRegister(addr, word))
            .await
            .map(|modbus_rsp| {
                modbus_rsp.map(|rsp| match rsp {
                    Response::WriteSingleRegister(rsp_addr, rsp_word) => {
                        debug_assert_eq!(addr, rsp_addr);
                        debug_assert_eq!(word, rsp_word);
                    }
                    others => {
                        // NOTE: A call to `Client::call` implementation *MUST* always return the `Response` variant matching the `Request` one.
                        // TIPS: This can be ensured via a call to `verify_response_header`( in 'src/service/mod.rs') before returning from `Client::call`.
                        unreachable!(
                            "{}",
                            unexpected_rsp_code_panic_msg(
                                FunctionCode::WriteSingleRegister,
                                others.function_code()
                            ),
                        )
                    }
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
            .map(|modbus_rsp| {
                modbus_rsp.map(|rsp| match rsp {
                    Response::WriteMultipleRegisters(rsp_addr, rsp_cnt) => {
                        debug_assert_eq!(addr, rsp_addr);
                        debug_assert_eq!(cnt, rsp_cnt.into());
                    }
                    others => {
                        // NOTE: A call to `Client::call` implementation *MUST* always return the `Response` variant matching the `Request` one.
                        // TIPS: This can be ensured via a call to `verify_response_header`( in 'src/service/mod.rs') before returning from `Client::call`.
                        unreachable!(
                            "{}",
                            unexpected_rsp_code_panic_msg(
                                FunctionCode::WriteMultipleRegisters,
                                others.function_code()
                            ),
                        )
                    }
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
            .map(|modbus_rsp| {
                modbus_rsp.map(|rsp| match rsp {
                    Response::MaskWriteRegister(rsp_addr, rsp_and_mask, rsp_or_mask) => {
                        debug_assert_eq!(addr, rsp_addr);
                        debug_assert_eq!(and_mask, rsp_and_mask);
                        debug_assert_eq!(or_mask, rsp_or_mask);
                    }
                    others => {
                        // NOTE: A call to `Client::call` implementation *MUST* always return the `Response` variant matching the `Request` one.
                        // TIPS: This can be ensured via a call to `verify_response_header`( in 'src/service/mod.rs') before returning from `Client::call`.
                        unreachable!(
                            "{}",
                            unexpected_rsp_code_panic_msg(
                                FunctionCode::MaskWriteRegister,
                                others.function_code()
                            ),
                        )
                    }
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

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
            match self.next_response.as_ref().unwrap() {
                Ok(response) => Ok(response.clone()),
                Err(err) => Err(io::Error::new(err.kind(), format!("{err}"))),
            }
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
