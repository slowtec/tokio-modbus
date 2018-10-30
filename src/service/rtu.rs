use crate::frame::*;
use futures::{future, Future};
use crate::proto::rtu::Proto;
use std::io::{Error, ErrorKind};
use tokio_core::reactor::Handle;
use tokio_proto::pipeline::ClientService;
use tokio_serial::Serial;
use tokio_service::Service;

/// Modbus RTU client
pub struct Client {
    address: u8,
    service: ClientService<Serial, Proto>,
}

use tokio_proto::BindClient;

impl Client {
    pub fn connect(
        serial: Serial,
        address: u8,
        handle: &Handle,
    ) -> Box<Future<Item = Client, Error = Error>> {
        let proto = Proto;
        let service = proto.bind_client(handle, serial);
        Box::new(future::ok(Client { address, service }))
    }
}

impl Service for Client {
    type Request = Request;
    type Response = Response;
    type Error = Error;
    type Future = Box<Future<Item = Response, Error = Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let pdu = Pdu::Request(req);
        let address = self.address;
        let req = RtuAdu { address, pdu };
        let result = self.service.call(req).and_then(move |resp| {
            if resp.address != address {
                return Err(Error::new(ErrorKind::InvalidData, "Invalid server ID"));
            }
            match resp.pdu {
                Pdu::Result(res) => match res {
                    Ok(pdu) => Ok(pdu),
                    Err(err) => Err(Error::new(ErrorKind::Other, err)),
                },
                _ => unreachable!(),
            }
        });
        Box::new(result)
    }
}
