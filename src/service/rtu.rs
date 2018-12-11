use crate::client::{Client, SwitchDevice};
use crate::device::*;
use crate::frame::{rtu::*, *};
use crate::proto::rtu::Proto;

use futures::{future, Future};
use std::io::{Error, ErrorKind};
use tokio_core::reactor::Handle;
use tokio_proto::pipeline::ClientService;
use tokio_proto::BindClient;
use tokio_serial::Serial;
use tokio_service::Service;

pub(crate) fn connect_device<D: Into<DeviceId>>(
    handle: &Handle,
    serial: Serial,
    device_id: D,
) -> impl Future<Item = Context, Error = Error> {
    let proto = Proto;
    let service = proto.bind_client(handle, serial);
    let device_id: DeviceId = device_id.into();
    let slave_addr = device_id.into();
    future::ok(Context {
        service,
        slave_addr,
    })
}

/// Modbus RTU client
pub(crate) struct Context {
    service: ClientService<Serial, Proto>,
    slave_addr: SlaveAddress,
}

impl Context {
    /// Establish a serial connection with a Modbus server.
    pub fn bind(
        handle: &Handle,
        serial: Serial,
        slave_addr: SlaveAddress,
    ) -> impl Future<Item = Self, Error = Error> {
        let proto = Proto;
        let service = proto.bind_client(handle, serial);
        future::ok(Self {
            service,
            slave_addr,
        })
    }

    fn next_request_adu<R>(&self, req: R) -> RequestAdu
    where
        R: Into<RequestPdu>,
    {
        let slave_addr = self.slave_addr;
        let hdr = Header { slave_addr };
        let pdu = req.into();
        RequestAdu { hdr, pdu }
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
        let res = self.slave_addr.into();
        self.slave_addr = device_id.into();
        res
    }
}

impl Client for Context {
    fn call(&self, req: Request) -> Box<dyn Future<Item = Response, Error = Error>> {
        Box::new(self.call_service(req))
    }
}
