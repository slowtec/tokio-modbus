use crate::frame::{tcp::*, *};
use crate::proto::tcp::Proto;

use futures::{future, Future};
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
/// Use 0xFF instead of 0x00 to distinguish direct connection messages from
/// broadcast messages to slave devices.
#[allow(dead_code)]
pub const DIRECT_CONNECTION_UNIT_ID: u8 = 0xFF;

/// The minimum Unit Identifier for addressing a slave device.
#[allow(dead_code)]
pub const MIN_SLAVE_DEVICE_UNIT_ID: u8 = super::MIN_ADDRESS;

/// The maximum Unit Identifier for addressing a slave device.
#[allow(dead_code)]
pub const MAX_SLAVE_DEVICE_UNIT_ID: u8 = super::MAX_ADDRESS;

/// Modbus TCP client
pub(crate) struct Client {
    service: ClientService<TcpStream, Proto>,
    transaction_id: Cell<u16>,
    unit_id: u8,
}

fn verify_slave_device_unit_id(slave_device_unit_id: u8) -> Result<u8, Error> {
    if slave_device_unit_id >= MIN_SLAVE_DEVICE_UNIT_ID
        && slave_device_unit_id <= MAX_SLAVE_DEVICE_UNIT_ID
    {
        Ok(slave_device_unit_id)
    } else {
        Err(Error::new(
            ErrorKind::Other,
            format!(
                "Invalid Modbus Unit Identifier for slave device: {}",
                slave_device_unit_id
            ),
        ))
    }
}

impl Client {
    /// Establish a direct connection with a Modbus server.
    pub fn connect(
        socket_addr: &SocketAddr,
        handle: &Handle,
    ) -> impl Future<Item = Client, Error = Error> {
        Self::connect_unit(socket_addr, handle, DIRECT_CONNECTION_UNIT_ID)
    }

    /// Establish an indirect connection with a slave device addressed
    /// through a Modbus Unit Identifier.
    #[allow(dead_code)]
    pub fn connect_slave_device(
        socket_addr: &SocketAddr,
        handle: &Handle,
        slave_device_unit_id: u8,
    ) -> Box<dyn Future<Item = Client, Error = Error>> {
        verify_slave_device_unit_id(slave_device_unit_id)
            .map(|unit_id| {
                let res: Box<dyn Future<Item = _, Error = _>> =
                    Box::new(Self::connect_unit(socket_addr, handle, unit_id));
                res
            })
            .unwrap_or_else(|err| Box::new(future::err(err)))
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

fn verify_response_header(req_hdr: Header, res_hdr: Header) -> Result<(), Error> {
    if req_hdr != res_hdr {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Invalid response header: expected/request = {:?}, actual/response = {:?}",
                req_hdr, res_hdr
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
            .and_then(move |res_adu| match res_adu.pdu {
                ResponsePdu(Ok(res)) => verify_response_header(req_hdr, res_adu.hdr).and(Ok(res)),
                ResponsePdu(Err(err)) => Err(Self::Error::new(ErrorKind::Other, err)),
            });

        Box::new(result)
    }
}
