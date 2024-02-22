// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{
    fmt, io,
    sync::atomic::{AtomicU16, Ordering},
};

use futures_util::{SinkExt as _, StreamExt as _};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

use crate::{
    codec,
    frame::{tcp::*, *},
    service::verify_response_header,
    slave::*,
    Result,
};

const INITIAL_TRANSACTION_ID: TransactionId = 0;

/// Modbus TCP client
#[derive(Debug)]
pub(crate) struct Client<T> {
    framed: Framed<T, codec::tcp::ClientCodec>,
    unit_id: UnitId,
    transaction_id: AtomicU16,
}

impl<T> Client<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) fn new(transport: T, slave: Slave) -> Self {
        let framed = Framed::new(transport, codec::tcp::ClientCodec::default());
        let unit_id: UnitId = slave.into();
        let transaction_id = AtomicU16::new(INITIAL_TRANSACTION_ID);
        Self {
            framed,
            unit_id,
            transaction_id,
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

    fn next_request_adu<'a, R>(&self, req: R, disconnect: bool) -> RequestAdu<'a>
    where
        R: Into<RequestPdu<'a>>,
    {
        RequestAdu {
            hdr: self.next_request_hdr(self.unit_id),
            pdu: req.into(),
            disconnect,
        }
    }

    pub(crate) async fn call(&mut self, req: Request<'_>) -> Result<Response> {
        log::debug!("Call {:?}", req);
        let disconnect = req == Request::Disconnect;
        let req_adu = self.next_request_adu(req, disconnect);
        let req_hdr = req_adu.hdr;

        self.framed.read_buffer_mut().clear();

        self.framed.send(req_adu).await?;
        let res_adu = self
            .framed
            .next()
            .await
            .ok_or_else(io::Error::last_os_error)??;

        match res_adu.pdu {
            ResponsePdu(Ok(res)) => verify_response_header(&req_hdr, &res_adu.hdr).and(Ok(Ok(res))),
            ResponsePdu(Err(err)) => Ok(Err(err.exception)),
        }
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
        Client::call(self, req).await
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
        assert!(matches!(
            result,
            Err(err) if err.kind() == std::io::ErrorKind::InvalidData));
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
        assert!(matches!(
            result,
            Err(err) if err.kind() == std::io::ErrorKind::InvalidData));
    }
}
