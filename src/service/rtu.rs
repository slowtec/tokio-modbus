// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{fmt, io};

use futures_util::{SinkExt as _, StreamExt as _};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

use crate::{
    codec,
    frame::{rtu::*, *},
    slave::*,
    ProtocolError, Result,
};

use super::{disconnect, verify_response_header};

/// Modbus RTU client
#[derive(Debug)]
pub(crate) struct Client<T> {
    framed: Option<Framed<T, codec::rtu::ClientCodec>>,
    slave_id: SlaveId,
}

impl<T> Client<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) fn new(transport: T, slave: Slave) -> Self {
        let framed = Framed::new(transport, codec::rtu::ClientCodec::default());
        let slave_id = slave.into();
        Self {
            slave_id,
            framed: Some(framed),
        }
    }

    fn framed(&mut self) -> io::Result<&mut Framed<T, codec::rtu::ClientCodec>> {
        let Some(framed) = &mut self.framed else {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "disconnected"));
        };
        Ok(framed)
    }

    fn next_request_adu<'a, R>(&self, req: R) -> RequestAdu<'a>
    where
        R: Into<RequestPdu<'a>>,
    {
        let slave_id = self.slave_id;
        let hdr = Header { slave_id };
        let pdu = req.into();
        RequestAdu { hdr, pdu }
    }

    async fn call(&mut self, req: Request<'_>) -> Result<Response> {
        log::debug!("Call {req:?}");

        let req_function_code = req.function_code();
        let req_adu = self.next_request_adu(req);
        let req_hdr = req_adu.hdr;

        let framed = self.framed()?;

        framed.read_buffer_mut().clear();
        framed.send(req_adu).await?;

        let res_adu = framed
            .next()
            .await
            .unwrap_or_else(|| Err(io::Error::from(io::ErrorKind::BrokenPipe)))?;
        let ResponseAdu {
            hdr: res_hdr,
            pdu: res_pdu,
        } = res_adu;
        let ResponsePdu(result) = res_pdu;

        // Match headers of request and response.
        if let Err(message) = verify_response_header(&req_hdr, &res_hdr) {
            return Err(ProtocolError::HeaderMismatch { message, result }.into());
        }

        // Match function codes of request and response.
        let rsp_function_code = match &result {
            Ok(response) => response.function_code(),
            Err(ExceptionResponse { function, .. }) => *function,
        };
        if req_function_code != rsp_function_code {
            return Err(ProtocolError::FunctionCodeMismatch {
                request: req_function_code,
                result,
            }
            .into());
        }

        Ok(result.map_err(
            |ExceptionResponse {
                 function: _,
                 exception,
             }| exception,
        ))
    }

    async fn disconnect(&mut self) -> io::Result<()> {
        let Some(framed) = self.framed.take() else {
            // Already disconnected.
            return Ok(());
        };
        disconnect(framed).await
    }
}

impl<T> SlaveContext for Client<T> {
    fn set_slave(&mut self, slave: Slave) {
        self.slave_id = slave.into();
    }
}

#[async_trait::async_trait]
impl<T> crate::client::Client for Client<T>
where
    T: fmt::Debug + AsyncRead + AsyncWrite + Send + Unpin,
{
    async fn call(&mut self, req: Request<'_>) -> Result<Response> {
        self.call(req).await
    }

    async fn disconnect(&mut self) -> io::Result<()> {
        self.disconnect().await
    }
}

#[cfg(test)]
mod tests {

    use core::{
        pin::Pin,
        task::{Context, Poll},
    };
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, Result};

    use crate::{
        service::{rtu::Header, verify_response_header},
        Error,
    };

    #[test]
    fn validate_same_headers() {
        // Given
        let req_hdr = Header { slave_id: 0 };
        let rsp_hdr = Header { slave_id: 0 };

        // When
        let result = verify_response_header(&req_hdr, &rsp_hdr);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_validate_not_same_slave_id() {
        // Given
        let req_hdr = Header { slave_id: 0 };
        let rsp_hdr = Header { slave_id: 5 };

        // When
        let result = verify_response_header(&req_hdr, &rsp_hdr);

        // Then
        assert!(result.is_err());
    }

    #[derive(Debug)]
    struct MockTransport;

    impl Unpin for MockTransport {}

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
        fn poll_write(self: Pin<&mut Self>, _: &mut Context<'_>, _: &[u8]) -> Poll<Result<usize>> {
            Poll::Ready(Ok(2))
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn handle_broken_pipe() {
        let transport = MockTransport;
        let mut client =
            crate::service::rtu::Client::new(transport, crate::service::rtu::Slave::broadcast());
        let res = client
            .call(crate::service::rtu::Request::ReadCoils(0x00, 5))
            .await;
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(
            matches!(err, Error::Transport(err) if err.kind() == std::io::ErrorKind::BrokenPipe)
        );
    }
}
