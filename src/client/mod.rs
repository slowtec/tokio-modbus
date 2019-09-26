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

use std::pin::Pin;

/// A transport independent asynchronous client trait.
pub trait Client: SlaveContext {
    //fn call(&self, request: Request) -> Pin<Box<dyn Future<Output = Response>>>;
    fn call<'a>(&'a mut self, request: Request) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + 'a>>;

}

/// An asynchronous Modbus reader.
pub trait Reader: Client {
    fn read_coils<'a>(
        &'a mut self,
        _: Address,
        _: Quantity,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Coil>, Error>> + 'a>>;

    fn read_discrete_inputs<'a>(
        &'a mut self,
        _: Address,
        _: Quantity,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Coil>, Error>> + 'a>>;

    fn read_input_registers<'a>(
        &'a mut self,
        _: Address,
        _: Quantity,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Word>, Error>> + 'a>>;

    fn read_holding_registers<'a>(
        &'a mut self,
        _: Address,
        _: Quantity,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Word>, Error>> + 'a>>;

    fn read_write_multiple_registers<'a>(
        &'a mut self,
        _: Address,
        _: Quantity,
        _: Address,
        _: &[Word],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Word>, Error>> + 'a>>;
}

/// An asynchronous Modbus writer.
pub trait Writer: Client {
    fn write_single_coil<'a>(&'a mut self, _: Address, _: Coil) -> Pin<Box<dyn Future<Output = Result<(), Error>> + 'a>>;

    fn write_multiple_coils<'a>(
        &'a mut self,
        _: Address,
        _: &[Coil],
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + 'a>>;

    fn write_single_register<'a>(
        &'a mut self,
        _: Address,
        _: Word,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + 'a>>;

    fn write_multiple_registers<'a>(
        &'a mut self,
        _: Address,
        _: &[Word],
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + 'a>>;
}

/// An asynchronous Modbus client context.
pub struct Context {
    client: Box<dyn Client>,
}


impl Context {
    pub async fn disconnect(&mut self) -> Result<(), Error> {
        // Disconnecting is expected to fail!
        let res = self.client.call(Request::Disconnect).await;
        match res {
            Ok(_) => unreachable!(),
            Err(err) => {
                match err.kind() {
                    ErrorKind::NotConnected | ErrorKind::BrokenPipe => Ok(()),
                    _ => Err(err),
                }
            },
        }
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
    fn call<'a>(&'a mut self, request: Request) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + 'a>> {
        self.client.call(request)
    }
}

impl SlaveContext for Context {
    fn set_slave(&mut self, slave: Slave) {
        self.client.set_slave(slave);
    }
}

impl Reader for Context {
    fn read_coils<'a>(
        &'a mut self,
        addr: Address,
        cnt: Quantity,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Coil>, Error>> + 'a>> {
        let request = self.client.call(Request::ReadCoils(addr, cnt));

        Box::pin(async move {
            let rsp = request.await?;

            if let Response::ReadCoils(mut coils) = rsp {
                debug_assert!(coils.len() >= cnt as usize);
                coils.truncate(cnt as usize);
                Ok(coils)
            } else {
                Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
            }
        })
    }

    fn read_discrete_inputs<'a>(
        &'a mut self,
        addr: Address,
        cnt: Quantity,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Coil>, Error>> + 'a>> {
        let request = self.client.call(Request::ReadDiscreteInputs(addr, cnt));

        Box::pin(async move {
            let rsp = request.await?;

            if let Response::ReadDiscreteInputs(mut coils) = rsp {
                debug_assert!(coils.len() >= cnt as usize);
                coils.truncate(cnt as usize);
                Ok(coils)
            } else {
                Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
            }    
        })
    }

    fn read_input_registers<'a>(
        &'a mut self,
        addr: Address,
        cnt: Quantity,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Word>, Error>> + 'a>> {
        let request = self.client.call(Request::ReadInputRegisters(addr, cnt));
        
        Box::pin(async move {
            let rsp = request.await?;

            if let Response::ReadInputRegisters(rsp) = rsp {
                if rsp.len() != cnt as usize {
                    return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                }
                Ok(rsp)
            } else {
                Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
            }
        })
    }

    fn read_holding_registers<'a>(
        &'a mut self,
        addr: Address,
        cnt: Quantity,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Word>, Error>> + 'a>> {
        let request = self.client.call(Request::ReadHoldingRegisters(addr, cnt));

        Box::pin(async move {
            let rsp = request.await?;
                
            if let Response::ReadHoldingRegisters(rsp) = rsp {
                if rsp.len() != cnt as usize {
                    return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                }
                Ok(rsp)
            } else {
                Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
            }
        })
    }

    fn read_write_multiple_registers<'a>(
        &'a mut self,
        read_addr: Address,
        read_cnt: Quantity,
        write_addr: Address,
        write_data: &[Word],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Word>, Error>> + 'a>> {
        let request = self.client
            .call(Request::ReadWriteMultipleRegisters(
                read_addr,
                read_cnt,
                write_addr,
                write_data.to_vec(),
            ));
        Box::pin(async move {
            let rsp = request.await?;

            if let Response::ReadWriteMultipleRegisters(rsp) = rsp {
                if rsp.len() != read_cnt as usize {
                    return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                }
                Ok(rsp)
            } else {
                Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
            }
        })
    }
}

impl Writer for Context {
    fn write_single_coil<'a>(
        &'a mut self,
        addr: Address,
        coil: Coil,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + 'a>> {
        let request = self.client.call(Request::WriteSingleCoil(addr, coil));

        Box::pin(async move {
            let rsp = request.await?;
          
            if let Response::WriteSingleCoil(rsp_addr) = rsp {
                if rsp_addr != addr {
                    return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                }
                Ok(())
            } else {
                Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
            }
        })
    }

    fn write_multiple_coils<'a>(
        &'a mut self,
        addr: Address,
        coils: &[Coil],
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + 'a>> {
        let cnt = coils.len();
        let request = self.client.call(Request::WriteMultipleCoils(addr, coils.to_vec()));

        Box::pin(async move {
            let rsp = request.await?;
                
            if let Response::WriteMultipleCoils(rsp_addr, rsp_cnt) = rsp {
                if rsp_addr != addr || rsp_cnt as usize != cnt {
                    return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                }
                Ok(())
            } else {
                Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
            }
        })
    }

    fn write_single_register<'a>(
        &'a mut self,
        addr: Address,
        data: Word,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + 'a>> {
        let request = self.client.call(Request::WriteSingleRegister(addr, data));

        Box::pin(async move {
            let rsp = request.await?;
    
            if let Response::WriteSingleRegister(rsp_addr, rsp_word) = rsp {
                if rsp_addr != addr || rsp_word != data {
                    return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                }
                Ok(())
            } else {
                Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
            }
        })
    }

    fn write_multiple_registers<'a>(
        &'a mut self,
        addr: Address,
        data: &[Word],
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + 'a>> {
        let cnt = data.len();
        let request = self.client.call(Request::WriteMultipleRegisters(addr, data.to_vec()));

        Box::pin(async move {
            let rsp = request.await?;
                
            if let Response::WriteMultipleRegisters(rsp_addr, rsp_cnt) = rsp {
                if rsp_addr != addr || rsp_cnt as usize != cnt {
                    return Err(Error::new(ErrorKind::InvalidData, "invalid response"));
                }
                Ok(())
            } else {
                Err(Error::new(ErrorKind::InvalidData, "unexpected response"))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::future;

    use std::cell::RefCell;

    #[derive(Default, Debug)]
    pub struct ClientMock {
        slave: Option<Slave>,
        last_request: RefCell<Option<Request>>,
        next_response: Option<Result<Response, Error>>,
    }

    #[allow(dead_code)]
    impl ClientMock {
        pub fn slave(&self) -> Option<Slave> {
            self.slave
        }

        pub fn last_request(&self) -> &RefCell<Option<Request>> {
            &self.last_request
        }

        pub fn set_next_response(&mut self, next_response: Result<Response, Error>) {
            self.next_response = Some(next_response);
        }
    }

    impl Client for ClientMock {
        fn call<'a>(&'a mut self, request: Request) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + 'a>> {
            self.last_request.replace(Some(request));
            Box::pin(future::ready(match self.next_response.as_ref().unwrap() {
                Ok(response) => Ok(response.clone()),
                Err(err) => Err(Error::new(err.kind(), format!("{}", err))),
            }))
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
        let response_coils = [true, false, false, true, false, true, false, true].to_vec();
        for num_coils in 1usize..8usize {
            let mut client = Box::new(ClientMock::default());
            client.set_next_response(Ok(Response::ReadCoils(response_coils.clone())));
            let mut context = Context { client };
            context.set_slave(Slave(1));
            let coils =  futures::executor::block_on(context.read_coils(1, num_coils as u16)).unwrap();
            assert_eq!(&response_coils[0..num_coils], &coils[..]);
        }
    }

    #[test]
    fn read_some_discrete_inputs() {
        // The protocol will always return entire bytes with, i.e.
        // a multiple of 8 coils.
        let response_inputs = [true, false, false, true, false, true, false, true].to_vec();
        for num_inputs in 1usize..8usize {
            let mut client = Box::new(ClientMock::default());
            client.set_next_response(Ok(Response::ReadDiscreteInputs(response_inputs.clone())));
            let mut context = Context { client };
            context.set_slave(Slave(1));
            let inputs = futures::executor::block_on(context
                .read_discrete_inputs(1, num_inputs as u16))
                .unwrap();
            assert_eq!(&response_inputs[0..num_inputs], &inputs[..]);
        }
    }
}
