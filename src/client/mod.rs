#[cfg(feature = "rtu")]
pub mod rtu;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "tcp")]
pub mod tcp;

use crate::frame::*;

use futures::prelude::*;
use std::io::{Error, ErrorKind};
use tokio_service::Service;

/// A transport independent asynchronous client trait.
pub trait Client {
    fn call(&self, req: Request) -> Box<dyn Future<Item = Response, Error = Error>>;
}

/// An asynchronous Modbus reader.
pub trait Reader {
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
pub trait Writer {
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

/// An asynchronous Modbus client connection.
pub struct Connection {
    service: Box<
        dyn Service<
            Request = Request,
            Response = Response,
            Error = Error,
            Future = Box<dyn Future<Item = Response, Error = Error>>,
        >,
    >,
}

impl Client for Connection {
    fn call(&self, req: Request) -> Box<dyn Future<Item = Response, Error = Error>> {
        self.service.call(req)
    }
}

impl Reader for Connection {
    fn read_coils(
        &self,
        addr: Address,
        cnt: Quantity,
    ) -> Box<dyn Future<Item = Vec<Coil>, Error = Error>> {
        Box::new(
            self.service
                .call(Request::ReadCoils(addr, cnt))
                .and_then(|res| {
                    if let Response::ReadCoils(coils) = res {
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
            self.service
                .call(Request::ReadDiscreteInputs(addr, cnt))
                .and_then(|res| {
                    if let Response::ReadDiscreteInputs(coils) = res {
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
            self.service
                .call(Request::ReadInputRegisters(addr, cnt))
                .and_then(move |res| {
                    if let Response::ReadInputRegisters(res) = res {
                        if res.len() != cnt as usize {
                            return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                        }
                        Ok(res)
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
            self.service
                .call(Request::ReadHoldingRegisters(addr, cnt))
                .and_then(move |res| {
                    if let Response::ReadHoldingRegisters(res) = res {
                        if res.len() != cnt as usize {
                            return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                        }
                        Ok(res)
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
            self.service
                .call(Request::ReadWriteMultipleRegisters(
                    read_addr,
                    read_cnt,
                    write_addr,
                    write_data.to_vec(),
                ))
                .and_then(move |res| {
                    if let Response::ReadWriteMultipleRegisters(res) = res {
                        if res.len() != read_cnt as usize {
                            return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                        }
                        Ok(res)
                    } else {
                        Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
                    }
                }),
        )
    }
}

impl Writer for Connection {
    fn write_single_coil(
        &self,
        addr: Address,
        coil: Coil,
    ) -> Box<dyn Future<Item = (), Error = Error>> {
        Box::new(
            self.service
                .call(Request::WriteSingleCoil(addr, coil))
                .and_then(move |res| {
                    if let Response::WriteSingleCoil(res_addr) = res {
                        if res_addr != addr {
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
            self.service
                .call(Request::WriteMultipleCoils(addr, coils.to_vec()))
                .and_then(move |res| {
                    if let Response::WriteMultipleCoils(res_addr, res_cnt) = res {
                        if res_addr != addr || res_cnt as usize != cnt {
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
            self.service
                .call(Request::WriteSingleRegister(addr, data))
                .and_then(move |res| {
                    if let Response::WriteSingleRegister(res_addr, res_word) = res {
                        if res_addr != addr || res_word != data {
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
            self.service
                .call(Request::WriteMultipleRegisters(addr, data.to_vec()))
                .and_then(move |res| {
                    if let Response::WriteMultipleRegisters(res_addr, res_cnt) = res {
                        if res_addr != addr || res_cnt as usize != cnt {
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
