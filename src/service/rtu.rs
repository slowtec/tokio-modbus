use crate::client::Client;
use crate::frame::{rtu::*, *};
use crate::proto::rtu::Proto;
use crate::slave::*;

use futures::{future, Future};
use std::io::{Error, ErrorKind};
use tokio_core::reactor::Handle;
use tokio_proto::pipeline::ClientService;
use tokio_proto::BindClient;
use tokio_serial::Serial;
use tokio_service::Service;

pub(crate) fn connect_slave(
    handle: &Handle,
    serial: Serial,
    slave: Slave,
) -> impl Future<Item = Context, Error = Error> {
    let proto = Proto;
    let service = proto.bind_client(handle, serial);
    let slave_id = slave.into();
    future::ok(Context { service, slave_id })
}

/// Modbus RTU client
pub(crate) struct Context {
    service: ClientService<Serial, Proto>,
    slave_id: SlaveId,
}

impl Context {
    /// Establish a serial connection with a Modbus server.
    pub fn bind(
        handle: &Handle,
        serial: Serial,
        slave_id: SlaveId,
    ) -> impl Future<Item = Self, Error = Error> {
        let proto = Proto;
        let service = proto.bind_client(handle, serial);
        future::ok(Self { service, slave_id })
    }

    fn next_request_adu<R>(&self, req: R) -> RequestAdu
    where
        R: Into<RequestPdu>,
    {
        let slave_id = self.slave_id;
        let hdr = Header { slave_id };
        let pdu = req.into();
        RequestAdu { hdr, pdu }
    }

    fn call(&self, req: Request) -> impl Future<Item = Response, Error = Error> {
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

impl SlaveContext for Context {
    fn set_slave(&mut self, slave: Slave) {
        self.slave_id = slave.into();
    }
}

impl Client for Context {
    fn call(&self, req: Request) -> Box<dyn Future<Item = Response, Error = Error>> {
        Box::new(self.call(req))
    }
}
