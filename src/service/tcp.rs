use crate::{
    client::Client,
    codec,
    frame::{tcp::*, *},
    slave::*,
};

use futures_util::{sink::SinkExt as _, stream::StreamExt as _};
use std::{
    future::Future,
    io::{Error, ErrorKind},
    net::SocketAddr,
    sync::atomic::{AtomicU16, Ordering},
};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

pub(crate) fn connect_slave(
    socket_addr: SocketAddr,
    slave: Slave,
) -> impl Future<Output = Result<Context, Error>> + 'static {
    let unit_id: UnitId = slave.into();
    async move {
        let service = TcpStream::connect(socket_addr).await?;
        let framed = Framed::new(service, codec::tcp::ClientCodec::default());

        let context: Context = Context::new(framed, unit_id);

        Ok(context)
    }
}

const INITIAL_TRANSACTION_ID: TransactionId = 0;

/// Modbus TCP client
#[derive(Debug)]
pub(crate) struct Context {
    service: Framed<TcpStream, codec::tcp::ClientCodec>,
    unit_id: UnitId,
    transaction_id: AtomicU16,
}

impl Context {
    fn new(service: Framed<TcpStream, codec::tcp::ClientCodec>, unit_id: UnitId) -> Self {
        Self {
            service,
            unit_id,
            transaction_id: AtomicU16::new(INITIAL_TRANSACTION_ID),
        }
    }

    fn next_transaction_id(&self) -> TransactionId {
        let transaction_id = self.transaction_id.load(Ordering::Relaxed);
        self.transaction_id
            .store(transaction_id.wrapping_add(1), Ordering::Relaxed);
        transaction_id
    }

    fn next_request_hdr(&self, unit_id: UnitId) -> Header {
        let transaction_id = self.next_transaction_id();
        Header {
            transaction_id,
            unit_id,
        }
    }

    fn next_request_adu<R>(&self, req: R, disconnect: bool) -> RequestAdu
    where
        R: Into<RequestPdu>,
    {
        RequestAdu {
            hdr: self.next_request_hdr(self.unit_id),
            pdu: req.into(),
            disconnect,
        }
    }

    pub async fn call(&mut self, req: Request) -> Result<Response, Error> {
        log::debug!("Call {:?}", req);
        let disconnect = req == Request::Disconnect;
        let req_adu = self.next_request_adu(req, disconnect);
        let req_hdr = req_adu.hdr;

        self.service.send(req_adu).await?;
        let res_adu = self
            .service
            .next()
            .await
            .ok_or_else(Error::last_os_error)??;

        match res_adu.pdu {
            ResponsePdu(Ok(res)) => verify_response_header(req_hdr, res_adu.hdr).and(Ok(res)),
            ResponsePdu(Err(err)) => Err(Error::new(ErrorKind::Other, err)),
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

impl SlaveContext for Context {
    fn set_slave(&mut self, slave: Slave) {
        self.unit_id = slave.into();
    }
}

#[async_trait::async_trait]
impl Client for Context {
    async fn call(&mut self, req: Request) -> Result<Response, Error> {
        Context::call(self, req).await
    }
}
