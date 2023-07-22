// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus clients

use std::{
    borrow::Cow,
    fmt::Debug,
    io::{Error, ErrorKind},
};

use async_trait::async_trait;

use crate::{frame::*, slave::*};

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
    async fn call(&mut self, request: Request<'_>) -> Result<Response, Error>;
}

/// Asynchronous Modbus reader
#[async_trait]
pub trait Reader: Client {
    /// Read multiple coils (0x01)
    async fn read_coils(&mut self, _: Address, _: Quantity) -> Result<Vec<Coil>, Error>;

    /// Read multiple discrete inputs (0x02)
    async fn read_discrete_inputs(&mut self, _: Address, _: Quantity) -> Result<Vec<Coil>, Error>;

    /// Read multiple holding registers (0x03)
    async fn read_holding_registers(&mut self, _: Address, _: Quantity)
        -> Result<Vec<Word>, Error>;

    /// Read multiple input registers (0x04)
    async fn read_input_registers(&mut self, _: Address, _: Quantity) -> Result<Vec<Word>, Error>;

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
    ) -> Result<Vec<Word>, Error>;
}

/// Asynchronous Modbus writer
#[async_trait]
pub trait Writer: Client {
    /// Write a single coil (0x05)
    async fn write_single_coil(&mut self, _: Address, _: Coil) -> Result<(), Error>;

    /// Write a single holding register (0x06)
    async fn write_single_register(&mut self, _: Address, _: Word) -> Result<(), Error>;

    /// Write multiple coils (0x0F)
    async fn write_multiple_coils(&mut self, addr: Address, data: &'_ [Coil]) -> Result<(), Error>;

    /// Write multiple holding registers (0x10)
    async fn write_multiple_registers(&mut self, addr: Address, data: &[Word])
        -> Result<(), Error>;

    /// Set or clear individual bits of a holding register (0x16)
    async fn masked_write_register(&mut self, _: Address, _: Word, _: Word) -> Result<(), Error>;
}

/// Asynchronous Modbus client context
#[derive(Debug)]
pub struct Context {
    client: Box<dyn Client>,
}

impl Context {
    /// Disconnect the client
    pub async fn disconnect(&mut self) -> Result<(), Error> {
        // Disconnecting is expected to fail!
        let res = self.client.call(Request::Disconnect).await;
        match res {
            Ok(_) => unreachable!(),
            Err(err) => match err.kind() {
                ErrorKind::NotConnected | ErrorKind::BrokenPipe => Ok(()),
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
    async fn call(&mut self, request: Request<'_>) -> Result<Response, Error> {
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
    async fn read_coils<'a>(
        &'a mut self,
        addr: Address,
        cnt: Quantity,
    ) -> Result<Vec<Coil>, Error> {
        let rsp = self.client.call(Request::ReadCoils(addr, cnt)).await?;

        if let Response::ReadCoils(mut coils) = rsp {
            debug_assert!(coils.len() >= cnt.into());
            coils.truncate(cnt.into());
            Ok(coils)
        } else {
            Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
        }
    }

    async fn read_discrete_inputs<'a>(
        &'a mut self,
        addr: Address,
        cnt: Quantity,
    ) -> Result<Vec<Coil>, Error> {
        let rsp = self
            .client
            .call(Request::ReadDiscreteInputs(addr, cnt))
            .await?;

        if let Response::ReadDiscreteInputs(mut coils) = rsp {
            debug_assert!(coils.len() >= cnt.into());
            coils.truncate(cnt.into());
            Ok(coils)
        } else {
            Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
        }
    }

    async fn read_input_registers<'a>(
        &'a mut self,
        addr: Address,
        cnt: Quantity,
    ) -> Result<Vec<Word>, Error> {
        let rsp = self
            .client
            .call(Request::ReadInputRegisters(addr, cnt))
            .await?;

        if let Response::ReadInputRegisters(rsp) = rsp {
            if rsp.len() != cnt.into() {
                return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
            }
            Ok(rsp)
        } else {
            Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
        }
    }

    async fn read_holding_registers<'a>(
        &'a mut self,
        addr: Address,
        cnt: Quantity,
    ) -> Result<Vec<Word>, Error> {
        let rsp = self
            .client
            .call(Request::ReadHoldingRegisters(addr, cnt))
            .await?;

        if let Response::ReadHoldingRegisters(rsp) = rsp {
            if rsp.len() != cnt.into() {
                return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
            }
            Ok(rsp)
        } else {
            Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
        }
    }

    async fn read_write_multiple_registers<'a>(
        &'a mut self,
        read_addr: Address,
        read_count: Quantity,
        write_addr: Address,
        write_data: &[Word],
    ) -> Result<Vec<Word>, Error> {
        let rsp = self
            .client
            .call(Request::ReadWriteMultipleRegisters(
                read_addr,
                read_count,
                write_addr,
                Cow::Borrowed(write_data),
            ))
            .await?;

        if let Response::ReadWriteMultipleRegisters(rsp) = rsp {
            if rsp.len() != read_count.into() {
                return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
            }
            Ok(rsp)
        } else {
            Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
        }
    }
}

#[async_trait]
impl Writer for Context {
    async fn write_single_coil<'a>(&'a mut self, addr: Address, coil: Coil) -> Result<(), Error> {
        let rsp = self
            .client
            .call(Request::WriteSingleCoil(addr, coil))
            .await?;

        if let Response::WriteSingleCoil(rsp_addr, rsp_coil) = rsp {
            if rsp_addr != addr || rsp_coil != coil {
                return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
            }
            Ok(())
        } else {
            Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
        }
    }

    async fn write_multiple_coils<'a>(
        &'a mut self,
        addr: Address,
        coils: &[Coil],
    ) -> Result<(), Error> {
        let cnt = coils.len();
        let rsp = self
            .client
            .call(Request::WriteMultipleCoils(addr, Cow::Borrowed(coils)))
            .await?;

        if let Response::WriteMultipleCoils(rsp_addr, rsp_cnt) = rsp {
            if rsp_addr != addr || usize::from(rsp_cnt) != cnt {
                return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
            }
            Ok(())
        } else {
            Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
        }
    }

    async fn write_single_register<'a>(
        &'a mut self,
        addr: Address,
        data: Word,
    ) -> Result<(), Error> {
        let rsp = self
            .client
            .call(Request::WriteSingleRegister(addr, data))
            .await?;

        if let Response::WriteSingleRegister(rsp_addr, rsp_word) = rsp {
            if rsp_addr != addr || rsp_word != data {
                return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
            }
            Ok(())
        } else {
            Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
        }
    }

    async fn write_multiple_registers<'a>(
        &'a mut self,
        addr: Address,
        data: &[Word],
    ) -> Result<(), Error> {
        let cnt = data.len();
        let rsp = self
            .client
            .call(Request::WriteMultipleRegisters(addr, Cow::Borrowed(data)))
            .await?;

        if let Response::WriteMultipleRegisters(rsp_addr, rsp_cnt) = rsp {
            if rsp_addr != addr || usize::from(rsp_cnt) != cnt {
                return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
            }
            Ok(())
        } else {
            Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
        }
    }

    async fn masked_write_register<'a>(
        &'a mut self,
        address: Address,
        and_mask: Word,
        or_mask: Word,
    ) -> Result<(), Error> {
        let rsp = self
            .client
            .call(Request::MaskWriteRegister(address, and_mask, or_mask))
            .await?;

        if let Response::MaskWriteRegister(addr, and, or) = rsp {
            if addr != address || and != and_mask || or != or_mask {
                return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
            }
            Ok(())
        } else {
            Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
        }
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
        next_response: Option<Result<Response, Error>>,
    }

    #[allow(dead_code)]
    impl ClientMock {
        pub(crate) fn slave(&self) -> Option<Slave> {
            self.slave
        }

        pub(crate) fn last_request(&self) -> &Mutex<Option<Request<'static>>> {
            &self.last_request
        }

        pub(crate) fn set_next_response(&mut self, next_response: Result<Response, Error>) {
            self.next_response = Some(next_response);
        }
    }

    #[async_trait]
    impl Client for ClientMock {
        async fn call(&mut self, request: Request<'_>) -> Result<Response, Error> {
            *self.last_request.lock().unwrap() = Some(request.into_owned());
            match self.next_response.as_ref().unwrap() {
                Ok(response) => Ok(response.clone()),
                Err(err) => Err(Error::new(err.kind(), format!("{err}"))),
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
            client.set_next_response(Ok(Response::ReadCoils(response_coils.to_vec())));
            let mut context = Context { client };
            context.set_slave(Slave(1));
            let coils = futures::executor::block_on(context.read_coils(1, num_coils)).unwrap();
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
            client.set_next_response(Ok(Response::ReadDiscreteInputs(response_inputs.to_vec())));
            let mut context = Context { client };
            context.set_slave(Slave(1));
            let inputs =
                futures::executor::block_on(context.read_discrete_inputs(1, num_inputs)).unwrap();
            assert_eq!(&response_inputs[0..num_inputs as usize], &inputs[..]);
        }
    }
}
