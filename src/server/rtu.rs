// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus RTU server skeleton

use std::{future::Future, io, path::Path};

use futures_util::{FutureExt as _, SinkExt as _, StreamExt as _};
use tokio_serial::SerialStream;
use tokio_util::codec::Framed;

use crate::{
    codec::rtu::ServerCodec,
    frame::{
        rtu::{RequestAdu, ResponseAdu},
        ExceptionResponse, OptionalResponsePdu, RequestPdu,
    },
};

use super::{Service, Terminated};

#[derive(Debug)]
pub struct Server {
    serial: SerialStream,
}

impl Server {
    /// set up a new [`Server`] instance from an interface path and baud rate
    pub fn new_from_path<P: AsRef<Path>>(p: P, baud_rate: u32) -> io::Result<Self> {
        let serial =
            SerialStream::open(&tokio_serial::new(p.as_ref().to_string_lossy(), baud_rate))?;
        Ok(Server { serial })
    }

    /// set up a new [`Server`] instance based on a pre-configured [`SerialStream`] instance
    #[must_use]
    pub fn new(serial: SerialStream) -> Self {
        Server { serial }
    }

    /// Process Modbus RTU requests.
    pub async fn serve_forever<S>(self, service: S) -> io::Result<()>
    where
        S: Service + Send + Sync + 'static,
        S::Request: From<RequestAdu<'static>> + Send,
    {
        let framed = Framed::new(self.serial, ServerCodec::default());
        process(framed, service).await
    }

    /// Process Modbus RTU requests until finished or aborted.
    ///
    /// Warning: Request processing is not scoped and could be aborted at any internal await point!
    /// See also: <https://rust-lang.github.io/wg-async/vision/roadmap/scopes.html#cancellation>
    pub async fn serve_until<S, X>(self, service: S, abort_signal: X) -> io::Result<Terminated>
    where
        S: Service + Send + Sync + 'static,
        S::Request: From<RequestAdu<'static>> + Send,
        X: Future<Output = ()> + Sync + Send + 'static,
    {
        let framed = Framed::new(self.serial, ServerCodec::default());
        let abort_signal = abort_signal.fuse();
        tokio::select! {
            res = process(framed, service) => {
                res.map(|()| Terminated::Finished)
            },
            () = abort_signal => {
                Ok(Terminated::Aborted)
            }
        }
    }
}

/// frame wrapper around the underlying service's responses to forwarded requests
async fn process<S>(mut framed: Framed<SerialStream, ServerCodec>, service: S) -> io::Result<()>
where
    S: Service + Send + Sync + 'static,
    S::Request: From<RequestAdu<'static>> + Send,
{
    loop {
        let Some(request_adu) = framed.next().await.transpose().inspect_err(|err| {
            log::debug!("Failed to receive and decode request ADU: {err}");
        })?
        else {
            log::debug!("Stream has finished");
            break;
        };

        let RequestAdu {
            hdr,
            pdu: RequestPdu(request),
        } = &request_adu;
        let hdr = *hdr;
        let fc = request.function_code();
        let OptionalResponsePdu(Some(response_pdu)) = service
            .call(request_adu.into())
            .await
            .map(Into::into)
            .map_err(|e| ExceptionResponse {
                function: fc,
                exception: e.into(),
            })
            .into()
        else {
            log::trace!("No response for request {hdr:?} (function = {fc})");
            continue;
        };

        framed
            .send(ResponseAdu {
                hdr,
                pdu: response_pdu,
            })
            .await
            .inspect_err(|err| {
                log::debug!("Failed to send response for request {hdr:?} (function = {fc}): {err}");
            })?;
    }
    Ok(())
}
