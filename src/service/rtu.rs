use crate::frame::{rtu::*, *};
use crate::proto::rtu::Proto;

use futures::{future, Future};
use std::io::{Error, ErrorKind};
use tokio_core::reactor::Handle;
use tokio_proto::pipeline::ClientService;
use tokio_proto::BindClient;
use tokio_serial::Serial;
use tokio_service::Service;

/// Modbus RTU client
pub struct Client {
    service: ClientService<Serial, Proto>,
    address: u8,
}

impl Client {
    /// Establish a serial connection with a Modbus server.
    pub fn connect(
        serial: Serial,
        address: u8,
        handle: &Handle,
    ) -> impl Future<Item = Client, Error = Error> {
        let proto = Proto;
        let service = proto.bind_client(handle, serial);
        future::ok(Client { service, address })
    }

    fn next_request_adu<R>(&self, req: R) -> RequestAdu
    where
        R: Into<RequestPdu>,
    {
        let address = self.address;
        let hdr = Header { address };
        let pdu = req.into();
        RequestAdu { hdr, pdu }
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
