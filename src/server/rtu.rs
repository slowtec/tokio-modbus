// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus RTU server skeleton

use std::{io, path::Path};

use futures::{Future, FutureExt as _};
use futures_util::{SinkExt as _, StreamExt as _};
use tokio_serial::SerialStream;
use tokio_util::codec::Framed;

use crate::{
    codec::rtu::ServerCodec,
    frame::{
        rtu::{RequestAdu, ResponseAdu},
        OptionalResponsePdu,
    },
    server::service::Service,
};

use super::Terminated;

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
        S::Response: Into<OptionalResponsePdu> + Send,
        S::Error: Into<io::Error>,
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
        S::Response: Into<OptionalResponsePdu> + Send,
        S::Error: Into<io::Error>,
        X: Future<Output = ()> + Sync + Send + Unpin + 'static,
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
async fn process<S, Req, Res>(
    mut framed: Framed<SerialStream, ServerCodec>,
    service: S,
) -> io::Result<()>
where
    S: Service<Request = Req, Response = Res> + Send + Sync + 'static,
    S::Request: From<RequestAdu<'static>> + Send,
    S::Response: Into<OptionalResponsePdu> + Send,
    S::Error: Into<io::Error>,
{
    loop {
        let Some(request) = framed.next().await.transpose()? else {
            log::debug!("Stream has finished");
            break;
        };

        let hdr = request.hdr;
        let OptionalResponsePdu(Some(response_pdu)) = service
            .call(request.into())
            .await
            .map_err(Into::into)?
            .into()
        else {
            log::debug!("Sending no response for request {hdr:?}");
            continue;
        };

        framed
            .send(ResponseAdu {
                hdr,
                pdu: response_pdu,
            })
            .await?;
    }
    Ok(())
}
