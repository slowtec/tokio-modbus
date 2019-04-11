#[cfg(feature = "rtu")]
pub mod rtu;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "tcp")]
pub mod tcp;

pub mod util;

use crate::frame::*;
use crate::slave::*;

use futures::prelude::*;
use std::io::{Error, ErrorKind};

/// A transport independent asynchronous client trait.
pub trait Client: SlaveContext {
    fn call(&self, request: Request) -> Box<dyn Future<Item = Response, Error = Error>>;
}

/// An asynchronous Modbus reader.
pub trait Reader: Client {
    fn read_coils(
        &self,
        _: Address,
        _: Quantity,
    ) -> Box<dyn Future<Item = Vec<Coil>, Error = Error>>;

    fn read_discrete_inputs(
        &self,
        _: Address,
        _: Quantity,
    ) -> Box<dyn Future<Item = Vec<Coil>, Error = Error>>;

    fn read_input_registers(
        &self,
        _: Address,
        _: Quantity,
    ) -> Box<dyn Future<Item = Vec<Word>, Error = Error>>;

    fn read_holding_registers(
        &self,
        _: Address,
        _: Quantity,
    ) -> Box<dyn Future<Item = Vec<Word>, Error = Error>>;

    fn read_write_multiple_registers(
        &self,
        _: Address,
        _: Quantity,
        _: Address,
        _: &[Word],
    ) -> Box<dyn Future<Item = Vec<Word>, Error = Error>>;
}

/// An asynchronous Modbus writer.
pub trait Writer: Client {
    fn write_single_coil(&self, _: Address, _: Coil) -> Box<dyn Future<Item = (), Error = Error>>;

    fn write_multiple_coils(
        &self,
        _: Address,
        _: &[Coil],
    ) -> Box<dyn Future<Item = (), Error = Error>>;

    fn write_single_register(
        &self,
        _: Address,
        _: Word,
    ) -> Box<dyn Future<Item = (), Error = Error>>;

    fn write_multiple_registers(
        &self,
        _: Address,
        _: &[Word],
    ) -> Box<dyn Future<Item = (), Error = Error>>;
}

/// An asynchronous Modbus client context.
pub struct Context {
    client: Box<dyn Client>,
}

impl Context {
    pub fn disconnect(&self) -> impl Future<Item = (), Error = Error> {
        // Disconnecting is expected to fail!
        self.client.call(Request::Disconnect).then(|res| match res {
            Ok(_) => unreachable!(),
            Err(err) => match err.kind() {
                ErrorKind::NotConnected | ErrorKind::BrokenPipe => Ok(()),
                _ => Err(err),
            },
        })
    }
}

impl From<Box<dyn Client>> for Context {
    fn from(client: Box<dyn Client>) -> Self {
        Self { client }
    }
}

impl Into<Box<dyn Client>> for Context {
    fn into(self) -> Box<dyn Client> {
        self.client
    }
}

impl Client for Context {
    fn call(&self, request: Request) -> Box<dyn Future<Item = Response, Error = Error>> {
        self.client.call(request)
    }
}

impl SlaveContext for Context {
    fn set_slave(&mut self, slave: Slave) {
        self.client.set_slave(slave);
    }
}

impl Reader for Context {
    fn read_coils(
        &self,
        addr: Address,
        cnt: Quantity,
    ) -> Box<dyn Future<Item = Vec<Coil>, Error = Error>> {
        Box::new(
            self.client
                .call(Request::ReadCoils(addr, cnt))
                .and_then(|rsp| {
                    if let Response::ReadCoils(coils) = rsp {
                        Ok(coils)
                    } else {
                        Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
                    }
                }),
        )
    }

    fn read_discrete_inputs(
        &self,
        addr: Address,
        cnt: Quantity,
    ) -> Box<dyn Future<Item = Vec<Coil>, Error = Error>> {
        Box::new(
            self.client
                .call(Request::ReadDiscreteInputs(addr, cnt))
                .and_then(|rsp| {
                    if let Response::ReadDiscreteInputs(coils) = rsp {
                        Ok(coils)
                    } else {
                        Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
                    }
                }),
        )
    }

    fn read_input_registers(
        &self,
        addr: Address,
        cnt: Quantity,
    ) -> Box<dyn Future<Item = Vec<Word>, Error = Error>> {
        Box::new(
            self.client
                .call(Request::ReadInputRegisters(addr, cnt))
                .and_then(move |rsp| {
                    if let Response::ReadInputRegisters(rsp) = rsp {
                        if rsp.len() != cnt as usize {
                            return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                        }
                        Ok(rsp)
                    } else {
                        Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
                    }
                }),
        )
    }

    fn read_holding_registers(
        &self,
        addr: Address,
        cnt: Quantity,
    ) -> Box<dyn Future<Item = Vec<Word>, Error = Error>> {
        Box::new(
            self.client
                .call(Request::ReadHoldingRegisters(addr, cnt))
                .and_then(move |rsp| {
                    if let Response::ReadHoldingRegisters(rsp) = rsp {
                        if rsp.len() != cnt as usize {
                            return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                        }
                        Ok(rsp)
                    } else {
                        Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
                    }
                }),
        )
    }

    fn read_write_multiple_registers(
        &self,
        read_addr: Address,
        read_cnt: Quantity,
        write_addr: Address,
        write_data: &[Word],
    ) -> Box<dyn Future<Item = Vec<Word>, Error = Error>> {
        Box::new(
            self.client
                .call(Request::ReadWriteMultipleRegisters(
                    read_addr,
                    read_cnt,
                    write_addr,
                    write_data.to_vec(),
                ))
                .and_then(move |rsp| {
                    if let Response::ReadWriteMultipleRegisters(rsp) = rsp {
                        if rsp.len() != read_cnt as usize {
                            return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                        }
                        Ok(rsp)
                    } else {
                        Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
                    }
                }),
        )
    }
}

impl Writer for Context {
    fn write_single_coil(
        &self,
        addr: Address,
        coil: Coil,
    ) -> Box<dyn Future<Item = (), Error = Error>> {
        Box::new(
            self.client
                .call(Request::WriteSingleCoil(addr, coil))
                .and_then(move |rsp| {
                    if let Response::WriteSingleCoil(rsp_addr) = rsp {
                        if rsp_addr != addr {
                            return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                        }
                        Ok(())
                    } else {
                        Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
                    }
                }),
        )
    }

    fn write_multiple_coils(
        &self,
        addr: Address,
        coils: &[Coil],
    ) -> Box<dyn Future<Item = (), Error = Error>> {
        let cnt = coils.len();
        Box::new(
            self.client
                .call(Request::WriteMultipleCoils(addr, coils.to_vec()))
                .and_then(move |rsp| {
                    if let Response::WriteMultipleCoils(rsp_addr, rsp_cnt) = rsp {
                        if rsp_addr != addr || rsp_cnt as usize != cnt {
                            return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                        }
                        Ok(())
                    } else {
                        Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
                    }
                }),
        )
    }

    fn write_single_register(
        &self,
        addr: Address,
        data: Word,
    ) -> Box<dyn Future<Item = (), Error = Error>> {
        Box::new(
            self.client
                .call(Request::WriteSingleRegister(addr, data))
                .and_then(move |rsp| {
                    if let Response::WriteSingleRegister(rsp_addr, rsp_word) = rsp {
                        if rsp_addr != addr || rsp_word != data {
                            return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                        }
                        Ok(())
                    } else {
                        Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
                    }
                }),
        )
    }

    fn write_multiple_registers(
        &self,
        addr: Address,
        data: &[Word],
    ) -> Box<dyn Future<Item = (), Error = Error>> {
        let cnt = data.len();
        Box::new(
            self.client
                .call(Request::WriteMultipleRegisters(addr, data.to_vec()))
                .and_then(move |rsp| {
                    if let Response::WriteMultipleRegisters(rsp_addr, rsp_cnt) = rsp {
                        if rsp_addr != addr || rsp_cnt as usize != cnt {
                            return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                        }
                        Ok(())
                    } else {
                        Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
                    }
                }),
        )
    }
}
