use futures::prelude::*;
use std::io::{Error, ErrorKind, Result};
use frame::*;
use tokio_service::Service;
use service;
use std::net::SocketAddr;
use tokio_core::reactor::Handle;
use tokio_serial::Serial;

/// A transport independent asynchronous client trait.
pub trait ModbusClient {
    fn read_coils(&self, Address, Quantity) -> Box<Future<Item = Vec<Coil>, Error = Error>>;
    fn read_discrete_inputs(
        &self,
        Address,
        Quantity,
    ) -> Box<Future<Item = Vec<Coil>, Error = Error>>;
    fn write_single_coil(&self, Address, Coil) -> Box<Future<Item = (), Error = Error>>;
    fn write_multiple_coils(&self, Address, &[Coil]) -> Box<Future<Item = (), Error = Error>>;
    fn read_input_registers(
        &self,
        Address,
        Quantity,
    ) -> Box<Future<Item = Vec<Word>, Error = Error>>;
    fn read_holding_registers(
        &self,
        Address,
        Quantity,
    ) -> Box<Future<Item = Vec<Word>, Error = Error>>;
    fn write_single_register(&self, Address, Word) -> Box<Future<Item = (), Error = Error>>;
    fn write_multiple_registers(&self, Address, &[Word]) -> Box<Future<Item = (), Error = Error>>;
    fn read_write_multiple_registers(
        &self,
        Address,
        Quantity,
        Address,
        &[Word],
    ) -> Box<Future<Item = Vec<Word>, Error = Error>>;
}

/// A transport independent synchronous client trait.
#[cfg(feature = "sync")]
pub trait SyncModbusClient {
    fn read_coils(&mut self, Address, Quantity) -> Result<Vec<Coil>>;
    fn read_discrete_inputs(&mut self, Address, Quantity) -> Result<Vec<Coil>>;
    fn write_single_coil(&mut self, Address, Coil) -> Result<()>;
    fn write_multiple_coils(&mut self, Address, &[Coil]) -> Result<()>;
    fn read_input_registers(&mut self, Address, Quantity) -> Result<Vec<Word>>;
    fn read_holding_registers(&mut self, Address, Quantity) -> Result<Vec<Word>>;
    fn write_single_register(&mut self, Address, Word) -> Result<()>;
    fn write_multiple_registers(&mut self, Address, &[Word]) -> Result<()>;
    fn read_write_multiple_registers(
        &mut self,
        Address,
        Quantity,
        Address,
        &[Word],
    ) -> Result<Vec<Word>>;
}

/// A modbus client implementation.
pub struct Client {
    transport: Box<
        Service<
            Request = Request,
            Response = Response,
            Error = Error,
            Future = Box<Future<Item = Response, Error = Error>>,
        >,
    >,
}

impl Client {
    pub fn connect_tcp(
        addr: &SocketAddr,
        handle: &Handle,
    ) -> Box<Future<Item = Client, Error = Error>> {
        let t = service::tcp::Client::connect(addr, handle).map(|c| Client {
            transport: Box::new(c),
        });
        Box::new(t)
    }
    pub fn connect_rtu(
        serial: Serial,
        address: u8,
        handle: &Handle,
    ) -> Box<Future<Item = Client, Error = Error>> {
        let t = service::rtu::Client::connect(serial, address, handle).map(|c| Client {
            transport: Box::new(c),
        });
        Box::new(t)
    }

    pub fn call(&self, req: Request) -> Box<Future<Item = Response, Error = Error>> {
        self.transport.call(req)
    }
}

impl ModbusClient for Client {
    fn read_coils(
        &self,
        addr: Address,
        cnt: Quantity,
    ) -> Box<Future<Item = Vec<Coil>, Error = Error>> {
        Box::new(self.call(Request::ReadCoils(addr, cnt)).and_then(|res| {
            if let Response::ReadCoils(coils) = res {
                Ok(coils)
            } else {
                Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
            }
        }))
    }

    fn read_discrete_inputs(
        &self,
        addr: Address,
        cnt: Quantity,
    ) -> Box<Future<Item = Vec<Coil>, Error = Error>> {
        Box::new(
            self.call(Request::ReadDiscreteInputs(addr, cnt))
                .and_then(|res| {
                    if let Response::ReadDiscreteInputs(coils) = res {
                        Ok(coils)
                    } else {
                        Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
                    }
                }),
        )
    }

    fn write_single_coil(
        &self,
        addr: Address,
        coil: Coil,
    ) -> Box<Future<Item = (), Error = Error>> {
        Box::new(
            self.call(Request::WriteSingleCoil(addr, coil))
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
    ) -> Box<Future<Item = (), Error = Error>> {
        let cnt = coils.len();
        Box::new(
            self.call(Request::WriteMultipleCoils(addr, coils.to_vec()))
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

    fn read_input_registers(
        &self,
        addr: Address,
        cnt: Quantity,
    ) -> Box<Future<Item = Vec<Word>, Error = Error>> {
        Box::new(
            self.call(Request::ReadInputRegisters(addr, cnt))
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
    ) -> Box<Future<Item = Vec<Word>, Error = Error>> {
        Box::new(
            self.call(Request::ReadHoldingRegisters(addr, cnt))
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

    fn write_single_register(
        &self,
        addr: Address,
        data: Word,
    ) -> Box<Future<Item = (), Error = Error>> {
        Box::new(
            self.call(Request::WriteSingleRegister(addr, data))
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
    ) -> Box<Future<Item = (), Error = Error>> {
        let cnt = data.len();
        Box::new(
            self.call(Request::WriteMultipleRegisters(addr, data.to_vec()))
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

    fn read_write_multiple_registers(
        &self,
        read_addr: Address,
        read_cnt: Quantity,
        write_addr: Address,
        write_data: &[Word],
    ) -> Box<Future<Item = Vec<Word>, Error = Error>> {
        Box::new(self.call(Request::ReadWriteMultipleRegisters(
            read_addr,
            read_cnt,
            write_addr,
            write_data.to_vec(),
        )).and_then(move |res| {
            if let Response::ReadWriteMultipleRegisters(res) = res {
                if res.len() != read_cnt as usize {
                    return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                }
                Ok(res)
            } else {
                Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
            }
        }))
    }
}
