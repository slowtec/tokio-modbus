use frame::*;
use futures::Future;
use proto::tcp::Proto;
use std::cell::Cell;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tokio_proto::pipeline::ClientService;
use tokio_proto::TcpClient;
use tokio_service::Service;

/// Modbus TCP client
pub struct Client {
    service: ClientService<TcpStream, Proto>,
    transaction_id: Cell<u16>,
    unit_id: u8,
}

impl Client {
    pub fn connect(
        addr: &SocketAddr,
        handle: &Handle,
    ) -> Box<Future<Item = Client, Error = Error>> {
        let client = TcpClient::new(Proto)
            .connect(addr, handle)
            .map(|client_service| Client {
                service: client_service,
                transaction_id: Cell::new(0),
                unit_id: 1,
            });
        Box::new(client)
    }
}

impl Service for Client {
    type Request = Request;
    type Response = Response;
    type Error = Error;
    type Future = Box<Future<Item = Response, Error = Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let t_id = self.transaction_id.get();
        let header = TcpHeader {
            transaction_id: t_id,
            unit_id: self.unit_id,
        };

        self.transaction_id.set(t_id.wrapping_add(1));

        let pdu = Pdu::Request(req);

        let result = self
            .service
            .call(TcpAdu { header, pdu })
            .and_then(move |adu| {
                if adu.header.transaction_id != t_id {
                    return Err(Error::new(ErrorKind::InvalidData, "Invalid transaction ID"));
                }
                match adu.pdu {
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
