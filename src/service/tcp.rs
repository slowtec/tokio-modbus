use crate::frame::{tcp::*, *};
use crate::proto::tcp::Proto;

use futures::Future;
use std::cell::Cell;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tokio_proto::pipeline::ClientService;
use tokio_proto::TcpClient;
use tokio_service::Service;

/// The Unit Identifier for direct connections
///
/// See also: [MODBUS Messaging on TCP/IP Implementation Guide](http://www.modbus.org/docs/Modbus_Messaging_Implementation_Guide_V1_0b.pdf), page 23
/// "On TCP/IP, the MODBUS server is addressed using its IP address; therefore,
/// the MODBUS Unit Identifier is useless. The value 0xFF has to be used."
/// "Remark: The value 0 is also accepted to communicate directly to a
/// MODBUS/TCP device."
///
/// Rationale: Use the proposed value 0xFF instead of the alternative
/// value 0x00 to distinguish direct connection messages from broadcast
/// messages that might be send to all slave devices connected to a
/// gateway!
const DIRECT_CONNECTION_UNIT_ID: u8 = 0xFF;

/// Modbus TCP client
pub(crate) struct Client {
    service: ClientService<TcpStream, Proto>,
    transaction_id: Cell<u16>,
    unit_id: u8,
}

impl Client {
    /// Establish a direct connection with a Modbus TCP server,
    /// i.e. not a gateway.
    pub fn connect(
        socket_addr: &SocketAddr,
        handle: &Handle,
    ) -> impl Future<Item = Client, Error = Error> {
        Self::connect_unit(socket_addr, handle, DIRECT_CONNECTION_UNIT_ID)
    }

    fn connect_unit(
        socket_addr: &SocketAddr,
        handle: &Handle,
        unit_id: u8,
    ) -> impl Future<Item = Client, Error = Error> {
        TcpClient::new(Proto)
            .connect(&socket_addr, &handle)
            .map(move |client_service| Client {
                service: client_service,
                transaction_id: Cell::new(0),
                unit_id,
            })
    }

    fn next_transaction_id(&self) -> u16 {
        let transaction_id = self.transaction_id.get();
        self.transaction_id.set(transaction_id.wrapping_add(1));
        transaction_id
    }

    fn next_request_hdr(&self) -> Header {
        let transaction_id = self.next_transaction_id();
        Header {
            transaction_id,
            unit_id: self.unit_id,
        }
    }

    fn next_request_adu<R>(&self, req: R) -> RequestAdu
    where
        R: Into<RequestPdu>,
    {
        RequestAdu {
            hdr: self.next_request_hdr(),
            pdu: req.into(),
        }
    }
}

fn verify_response_header(req_hdr: Header, rsp_hdr: Header) -> Result<(), Error> {
    if req_hdr != rsp_hdr {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Invalid response header: expected/request = {:?}, actual/response = {:?}",
                req_hdr, rsp_hdr
            ),
        ));
    }
    Ok(())
}

impl Service for Client {
    type Request = Request;
    type Response = Response;
    type Error = Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let req_adu = self.next_request_adu(req);
        let req_hdr = req_adu.hdr;

        let result = self
            .service
            .call(req_adu)
            .and_then(move |rsp_adu| match rsp_adu.pdu {
                ResponsePdu(Ok(rsp)) => verify_response_header(req_hdr, rsp_adu.hdr).and(Ok(rsp)),
                ResponsePdu(Err(err)) => Err(Self::Error::new(ErrorKind::Other, err)),
            });

        Box::new(result)
    }
}
