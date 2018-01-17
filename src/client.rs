use futures::prelude::*;
use std::io::{Error, ErrorKind};
use frame::*;
use tokio_service::Service;

/// A transport independent client trait.
pub trait Client {
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

impl<T> Client for T
where T: Service<
    Request = Request,
    Response = Response,
    Error = Error,
    Future = Box<Future<Item = Response, Error = Error>>
>
{
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
