use crate::{
    client::Client,
    codec,
    frame::{rtu::*, *},
    slave::*,
};

use futures_util::{future, sink::SinkExt as _, stream::StreamExt as _};
use std::{
    fmt::Debug,
    future::Future,
    io::{Error, ErrorKind},
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

pub(crate) fn connect_slave<T>(
    transport: T,
    slave: Slave,
) -> impl Future<Output = Result<Context<T>, Error>>
where
    T: AsyncRead + AsyncWrite + Debug + Unpin + 'static,
{
    let framed = Framed::new(transport, codec::rtu::ClientCodec::default());

    let slave_id = slave.into();
    future::ok(Context {
        service: framed,
        slave_id,
    })
}

/// Modbus RTU client
#[derive(Debug)]
pub(crate) struct Context<T: AsyncRead + AsyncWrite + Debug + Unpin + 'static> {
    service: Framed<T, codec::rtu::ClientCodec>,
    slave_id: SlaveId,
}

impl<T: AsyncRead + AsyncWrite + Unpin + Debug + 'static> Context<T> {
    fn next_request_adu<R>(&self, req: R, disconnect: bool) -> RequestAdu
    where
        R: Into<RequestPdu>,
    {
        let slave_id = self.slave_id;
        let hdr = Header { slave_id };
        let pdu = req.into();
        RequestAdu {
            hdr,
            pdu,
            disconnect,
        }
    }

    async fn call(&mut self, req: Request) -> Result<Response, Error> {
        let disconnect = req == Request::Disconnect;
        let req_adu = self.next_request_adu(req, disconnect);
        let req_hdr = req_adu.hdr;

        self.service.send(req_adu).await?;
        let res_adu = self
            .service
            .next()
            .await
            .unwrap_or_else(|| Err(Error::from(ErrorKind::BrokenPipe)))?;

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

impl<T: AsyncRead + AsyncWrite + Debug + Unpin + 'static> SlaveContext for Context<T> {
    fn set_slave(&mut self, slave: Slave) {
        self.slave_id = slave.into();
    }
}

#[async_trait::async_trait]
impl<T: AsyncRead + AsyncWrite + Debug + Unpin + Send + 'static> Client for Context<T> {
    async fn call(&mut self, req: Request) -> Result<Response, Error> {
        self.call(req).await
    }
}

#[cfg(test)]
mod tests {

    use core::{
        pin::Pin,
        task::{Context, Poll},
    };
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, Result};

    #[derive(Debug)]
    struct MockTransport;

    impl Unpin for MockTransport {}

    #[tokio::test]
    async fn handle_broken_pipe() {
        impl AsyncRead for MockTransport {
            fn poll_read(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
                _: &mut ReadBuf<'_>,
            ) -> Poll<Result<()>> {
                Poll::Ready(Ok(()))
            }
        }

        impl AsyncWrite for MockTransport {
            fn poll_write(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
                _: &[u8],
            ) -> Poll<Result<usize>> {
                Poll::Ready(Ok(2))
            }

            fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
                Poll::Ready(Ok(()))
            }

            fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
                unimplemented!()
            }
        }

        let transport = MockTransport {};
        let mut ctx =
            crate::service::rtu::connect_slave(transport, crate::service::rtu::Slave::broadcast())
                .await
                .unwrap();
        let res = ctx
            .call(crate::service::rtu::Request::ReadCoils(0x00, 5))
            .await;
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
    }
}
