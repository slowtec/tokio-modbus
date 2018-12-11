use crate::client::{Client, SwitchDevice};
use crate::device::*;
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
pub(crate) const DIRECT_CONNECTION_UNIT_ID: UnitId = 0xFF;

pub const DIRECT_CONNECTION_DEVICE_ID: DeviceId = DeviceId(DIRECT_CONNECTION_UNIT_ID);

pub(crate) fn connect_device<D: Into<DeviceId>>(
    handle: &Handle,
    socket_addr: SocketAddr,
    device_id: D,
) -> impl Future<Item = Context, Error = Error> {
    let device_id: DeviceId = device_id.into();
    let unit_id = device_id.into();
    TcpClient::new(Proto)
        .connect(&socket_addr, &handle)
        .map(move |service| Context::new(service, unit_id))
}

const INITIAL_TRANSACTION_ID: TransactionId = 0;

/// Modbus TCP client
pub(crate) struct Context {
    service: ClientService<TcpStream, Proto>,
    unit_id: UnitId,
    transaction_id: Cell<TransactionId>,
}

impl Context {
    fn new(service: ClientService<TcpStream, Proto>, unit_id: UnitId) -> Self {
        Self {
            service,
            unit_id,
            transaction_id: Cell::new(INITIAL_TRANSACTION_ID),
        }
    }

    fn next_transaction_id(&self) -> TransactionId {
        let transaction_id = self.transaction_id.get();
        self.transaction_id.set(transaction_id.wrapping_add(1));
        transaction_id
    }

    fn next_request_hdr(&self, unit_id: UnitId) -> Header {
        let transaction_id = self.next_transaction_id();
        Header {
            transaction_id,
            unit_id,
        }
    }

    fn next_request_adu<R>(&self, req: R) -> RequestAdu
    where
        R: Into<RequestPdu>,
    {
        RequestAdu {
            hdr: self.next_request_hdr(self.unit_id),
            pdu: req.into(),
        }
    }

    fn call_service(&self, req: Request) -> impl Future<Item = Response, Error = Error> {
        let req_adu = self.next_request_adu(req);
        let req_hdr = req_adu.hdr;
        self.service
            .call(req_adu)
            .and_then(move |res_adu| match res_adu.pdu {
                ResponsePdu(Ok(res)) => verify_response_header(req_hdr, res_adu.hdr).and(Ok(res)),
                ResponsePdu(Err(err)) => Err(Error::new(ErrorKind::Other, err)),
            })
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

impl SwitchDevice for Context {
    fn switch_device(&mut self, device_id: DeviceId) -> DeviceId {
        let res = self.unit_id.into();
        self.unit_id = device_id.into();
        res
    }
}

impl Client for Context {
    fn call(&self, req: Request) -> Box<dyn Future<Item = Response, Error = Error>> {
        Box::new(self.call_service(req))
    }
}
