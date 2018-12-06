#[cfg(feature = "rtu")]
pub mod rtu;

#[cfg(feature = "tcp")]
pub mod tcp;

use crate::frame::*;

use futures::prelude::*;
use std::io::{Error, ErrorKind};
use tokio_core::reactor::Handle;
use tokio_service::Service;

#[cfg(feature = "rtu")]
use std::io::Result;

#[cfg(feature = "rtu")]
use tokio_serial::{Serial, SerialPortSettings};

#[cfg(feature = "rtu")]
use tokio_core::reactor::Core;

#[cfg(feature = "tcp")]
use std::net::SocketAddr;

/// A transport independent asynchronous client trait.
pub trait ModbusClient {
    fn call(&self, req: Request) -> Box<dyn Future<Item = Response, Error = Error>>;

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

/// A transport independent synchronous client trait.
#[cfg(feature = "sync")]
pub trait SyncModbusClient {
    fn call(&mut self, req: Request) -> Result<Response>;

    fn read_coils(&mut self, _: Address, _: Quantity) -> Result<Vec<Coil>>;
    fn read_discrete_inputs(&mut self, _: Address, _: Quantity) -> Result<Vec<Coil>>;
    fn read_input_registers(&mut self, _: Address, _: Quantity) -> Result<Vec<Word>>;
    fn read_holding_registers(&mut self, _: Address, _: Quantity) -> Result<Vec<Word>>;
    fn read_write_multiple_registers(
        &mut self,
        _: Address,
        _: Quantity,
        _: Address,
        _: &[Word],
    ) -> Result<Vec<Word>>;

    fn write_single_coil(&mut self, _: Address, _: Coil) -> Result<()>;
    fn write_multiple_coils(&mut self, _: Address, _: &[Coil]) -> Result<()>;
    fn write_single_register(&mut self, _: Address, _: Word) -> Result<()>;
    fn write_multiple_registers(&mut self, _: Address, _: &[Word]) -> Result<()>;
}

/// A async modbus client implementation.
pub struct Client {
    service: Box<
        Service<
            Request = Request,
            Response = Response,
            Error = Error,
            Future = Box<dyn Future<Item = Response, Error = Error>>,
        >,
    >,
}

/// A sync modbus client implementation.
#[cfg(feature = "sync")]
pub struct SyncClient {
    async_client: Client,
    core: Core,
}

impl Client {
    #[cfg(feature = "tcp")]
    pub fn connect_tcp(
        socket_addr: &SocketAddr,
        handle: &Handle,
    ) -> Box<dyn Future<Item = Self, Error = Error>> {
        Box::new(self::tcp::connect(socket_addr, handle))
    }

    #[cfg(feature = "rtu")]
    pub fn connect_rtu(
        serial: Serial,
        address: u8,
        handle: &Handle,
    ) -> Box<dyn Future<Item = Client, Error = Error>> {
        Box::new(self::rtu::connect(serial, address, handle))
    }
}

#[cfg(feature = "sync")]
impl SyncClient {
    #[cfg(feature = "tcp")]
    pub fn connect_tcp(socket_addr: &SocketAddr) -> Result<SyncClient> {
        let mut core = Core::new()?;
        let handle = core.handle();
        let async_client = core.run(Client::connect_tcp(socket_addr, &handle))?;
        Ok(SyncClient { async_client, core })
    }
    #[cfg(feature = "rtu")]
    pub fn connect_rtu(
        tty_path: &str,
        settings: &SerialPortSettings,
        address: u8,
    ) -> Result<SyncClient> {
        let mut core = Core::new()?;
        let handle = core.handle();
        let serial = Serial::from_path_with_handle(tty_path, settings, &handle.new_tokio_handle())?;
        let async_client = core.run(Client::connect_rtu(serial, address, &handle))?;
        Ok(SyncClient { async_client, core })
    }
}

impl ModbusClient for Client {
    fn call(&self, req: Request) -> Box<dyn Future<Item = Response, Error = Error>> {
        self.service.call(req)
    }

    fn read_coils(
        &self,
        addr: Address,
        cnt: Quantity,
    ) -> Box<dyn Future<Item = Vec<Coil>, Error = Error>> {
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
    ) -> Box<dyn Future<Item = Vec<Coil>, Error = Error>> {
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
    ) -> Box<dyn Future<Item = (), Error = Error>> {
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
    ) -> Box<dyn Future<Item = (), Error = Error>> {
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
    ) -> Box<dyn Future<Item = Vec<Word>, Error = Error>> {
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
    ) -> Box<dyn Future<Item = Vec<Word>, Error = Error>> {
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
    ) -> Box<dyn Future<Item = (), Error = Error>> {
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
    ) -> Box<dyn Future<Item = (), Error = Error>> {
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
    ) -> Box<dyn Future<Item = Vec<Word>, Error = Error>> {
        Box::new(
            self.call(Request::ReadWriteMultipleRegisters(
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

#[cfg(feature = "sync")]
impl SyncModbusClient for SyncClient {
    fn call(&mut self, req: Request) -> Result<Response> {
        self.core.run(self.async_client.call(req))
    }

    fn read_coils(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Coil>> {
        self.core.run(self.async_client.read_coils(addr, cnt))
    }
    fn read_discrete_inputs(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Coil>> {
        self.core
            .run(self.async_client.read_discrete_inputs(addr, cnt))
    }
    fn write_single_coil(&mut self, addr: Address, coil: Coil) -> Result<()> {
        self.core
            .run(self.async_client.write_single_coil(addr, coil))
    }
    fn write_multiple_coils(&mut self, addr: Address, coils: &[Coil]) -> Result<()> {
        self.core
            .run(self.async_client.write_multiple_coils(addr, coils))
    }
    fn read_input_registers(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Word>> {
        self.core
            .run(self.async_client.read_input_registers(addr, cnt))
    }
    fn read_holding_registers(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Word>> {
        self.core
            .run(self.async_client.read_holding_registers(addr, cnt))
    }
    fn write_single_register(&mut self, addr: Address, data: Word) -> Result<()> {
        self.core
            .run(self.async_client.write_single_register(addr, data))
    }
    fn write_multiple_registers(&mut self, addr: Address, data: &[Word]) -> Result<()> {
        self.core
            .run(self.async_client.write_multiple_registers(addr, data))
    }
    fn read_write_multiple_registers(
        &mut self,
        read_addr: Address,
        read_cnt: Quantity,
        write_addr: Address,
        write_data: &[Word],
    ) -> Result<Vec<Word>> {
        self.core.run(
            self.async_client
                .read_write_multiple_registers(read_addr, read_cnt, write_addr, write_data),
        )
    }
}
