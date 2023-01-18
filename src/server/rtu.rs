// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus RTU server skeleton

use crate::{
    codec,
    frame::*,
    server::service::{NewService, Service},
};
use futures::{select, Future, FutureExt as _};
use futures_util::{SinkExt as _, StreamExt as _};
use log::debug;
use std::{io::Error, path::Path};
use tokio_serial::SerialStream;
use tokio_util::codec::Framed;

#[derive(Debug)]
pub struct Server {
    serial: SerialStream,
}

impl Server {
    /// set up a new Server instance from an interface path and baud rate
    pub fn new_from_path<P: AsRef<Path>>(p: P, baud_rate: u32) -> Result<Self, Error> {
        let serial =
            SerialStream::open(&tokio_serial::new(p.as_ref().to_string_lossy(), baud_rate))?;
        Ok(Server { serial })
    }

    /// set up a new Server instance based on a pre-configured SerialStream instance
    #[must_use]
    pub fn new(serial: SerialStream) -> Self {
        Server { serial }
    }

    /// serve Modbus RTU requests based on the provided service until it finishes
    pub async fn serve_forever<S, Req, Res>(self, new_service: S)
    where
        S: NewService<Request = Req, Response = Res> + Send + Sync + 'static,
        Req: From<rtu::RequestAdu> + Send,
        Res: TryInto<ResponsePdu> + Send,
        <Res as TryInto<ResponsePdu>>::Error: Send,
        S::Instance: Send + Sync + 'static,
        S::Error: Into<Error>,
    {
        self.serve_until(new_service, futures::future::pending())
            .await;
    }

    /// serve Modbus RTU requests based on the provided service until it finishes or a shutdown signal is received
    pub async fn serve_until<S, Req, Res, Sd>(self, new_service: S, shutdown_signal: Sd)
    where
        S: NewService<Request = Req, Response = Res> + Send + Sync + 'static,
        Sd: Future<Output = ()> + Sync + Send + Unpin + 'static,
        Req: From<rtu::RequestAdu> + Send,
        Res: TryInto<ResponsePdu> + Send,
        <Res as TryInto<ResponsePdu>>::Error: Send,
        S::Instance: Send + Sync + 'static,
        S::Error: Into<Error>,
    {
        let framed = Framed::new(self.serial, codec::rtu::ServerCodec::default());
        let service = new_service.new_service().unwrap();
        let future = process(framed, service);

        let mut server = Box::pin(future).fuse();
        let mut shutdown = shutdown_signal.fuse();

        async {
            select! {
                res = server => if let Err(e) = res {
                    println!("error: {e}");
                },
                _ = shutdown => println!("Shutdown signal received")
            }
        }
        .await;
    }
}

/// frame wrapper around the underlying service's responses to forwarded requests
async fn process<S, Req, Res>(
    mut framed: Framed<SerialStream, codec::rtu::ServerCodec>,
    service: S,
) -> Result<(), Error>
where
    S: Service<Request = Req, Response = Res> + Send + Sync + 'static,
    S::Request: From<rtu::RequestAdu> + Send,
    S::Response: TryInto<ResponsePdu> + Send,
    S::Error: Into<Error>,
{
    loop {
        let request = match framed.next().await {
            // Stream is exhausted
            None => break,
            Some(request) => request,
        }?;

        let hdr = request.hdr;
        let response = service.call(request.into()).await.map_err(Into::into)?;

        match response.try_into() {
            Ok(pdu) => {
                framed.send(rtu::ResponseAdu { hdr, pdu }).await?;
            }
            Err(_) => {
                debug!("skipping reponse");
            }
        }
    }
    Ok(())
}
