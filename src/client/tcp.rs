// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! TCP client connections

use std::{fmt, io, net::SocketAddr};

use futures_util::{SinkExt as _, StreamExt as _};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_util::codec::Framed;

use crate::{
    codec::tcp::ClientCodec,
    frame::{
        tcp::{Header, RequestAdu, ResponseAdu, TransactionId, UnitId},
        verify_response_header, RequestPdu, ResponsePdu,
    },
    slave::SlaveContext,
    ExceptionResponse, ProtocolError, Request, Response, Result, Slave,
};

use super::{disconnect_framed, Context};

/// Establish a direct connection to a Modbus TCP coupler.
pub async fn connect(socket_addr: SocketAddr) -> io::Result<Context> {
    connect_slave(socket_addr, Slave::tcp_device()).await
}

/// Connect to a physical, broadcast, or custom Modbus device,
/// probably through a Modbus TCP gateway that is forwarding
/// messages to/from the corresponding slave device.
pub async fn connect_slave(socket_addr: SocketAddr, slave: Slave) -> io::Result<Context> {
    let transport = TcpStream::connect(socket_addr).await?;
    let context = attach_slave(transport, slave);
    Ok(context)
}

/// Attach a new client context to a direct transport connection.
///
/// The connection could either be an ordinary [`TcpStream`] or a TLS connection.
pub fn attach<T>(transport: T) -> Context
where
    T: AsyncRead + AsyncWrite + Send + Unpin + fmt::Debug + 'static,
{
    attach_slave(transport, Slave::tcp_device())
}

/// Attach a new client context to a transport connection.
///
/// The connection could either be an ordinary [`TcpStream`] or a TLS connection.
pub fn attach_slave<T>(transport: T, slave: Slave) -> Context
where
    T: AsyncRead + AsyncWrite + Send + Unpin + fmt::Debug + 'static,
{
    let client = Client::new(transport, slave);
    Context {
        client: Box::new(client),
    }
}

const INITIAL_TRANSACTION_ID: TransactionId = 0;

#[derive(Debug)]
struct TransactionIdGenerator {
    next_transaction_id: TransactionId,
}

impl TransactionIdGenerator {
    const fn new() -> Self {
        Self {
            next_transaction_id: INITIAL_TRANSACTION_ID,
        }
    }

    fn next(&mut self) -> TransactionId {
        let next_transaction_id = self.next_transaction_id;
        self.next_transaction_id = next_transaction_id.wrapping_add(1);
        next_transaction_id
    }
}

/// Modbus TCP client
#[derive(Debug)]
pub(crate) struct Client<T> {
    framed: Option<Framed<T, ClientCodec>>,
    transaction_id_generator: TransactionIdGenerator,
    unit_id: UnitId,
}

impl<T> Client<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) fn new(transport: T, slave: Slave) -> Self {
        let framed = Framed::new(transport, ClientCodec::new());
        let transaction_id_generator = TransactionIdGenerator::new();
        let unit_id: UnitId = slave.into();
        Self {
            framed: Some(framed),
            transaction_id_generator,
            unit_id,
        }
    }

    fn next_request_hdr(&mut self, unit_id: UnitId) -> Header {
        let transaction_id = self.transaction_id_generator.next();
        Header {
            transaction_id,
            unit_id,
        }
    }

    fn next_request_adu<'a, R>(&mut self, req: R) -> RequestAdu<'a>
    where
        R: Into<RequestPdu<'a>>,
    {
        RequestAdu {
            hdr: self.next_request_hdr(self.unit_id),
            pdu: req.into(),
        }
    }

    fn framed(&mut self) -> io::Result<&mut Framed<T, ClientCodec>> {
        let Some(framed) = &mut self.framed else {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "disconnected"));
        };
        Ok(framed)
    }

    pub(crate) async fn call(&mut self, req: Request<'_>) -> Result<Response> {
        log::debug!("Call {req:?}");

        let req_function_code = req.function_code();
        let req_adu = self.next_request_adu(req);
        let req_hdr = req_adu.hdr;

        let framed = self.framed()?;

        framed.read_buffer_mut().clear();
        framed.send(req_adu).await?;

        let res_adu = framed.next().await.ok_or_else(io::Error::last_os_error)??;
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
        disconnect_framed(framed).await
    }
}

impl<T> SlaveContext for Client<T> {
    fn set_slave(&mut self, slave: Slave) {
        self.unit_id = slave.into();
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
    use super::*;

    #[test]
    fn validate_same_headers() {
        // Given
        let req_hdr = Header {
            unit_id: 0,
            transaction_id: 42,
        };
        let rsp_hdr = Header {
            unit_id: 0,
            transaction_id: 42,
        };

        // When
        let result = verify_response_header(&req_hdr, &rsp_hdr);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_validate_not_same_unit_id() {
        // Given
        let req_hdr = Header {
            unit_id: 0,
            transaction_id: 42,
        };
        let rsp_hdr = Header {
            unit_id: 5,
            transaction_id: 42,
        };

        // When
        let result = verify_response_header(&req_hdr, &rsp_hdr);

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn invalid_validate_not_same_transaction_id() {
        // Given
        let req_hdr = Header {
            unit_id: 0,
            transaction_id: 42,
        };
        let rsp_hdr = Header {
            unit_id: 0,
            transaction_id: 86,
        };

        // When
        let result = verify_response_header(&req_hdr, &rsp_hdr);

        // Then
        assert!(result.is_err());
    }
}
