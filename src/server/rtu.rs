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
    pub async fn serve_forever<S, Req>(self, new_service: S)
    where
        S: NewService<Request = Req, Response = Response> + Send + Sync + 'static,
        S::Error: Into<Error>,
        S::Instance: 'static + Send + Sync,
        S::Request: Into<Req>,
        S::Request: From<rtu::RequestAdu>,
        S::Instance: Send + Sync + 'static,
    {
        self.serve_until(new_service, futures::future::pending())
            .await;
    }

    /// serve Modbus RTU requests based on the provided service until it finishes or a shutdown signal is received
    pub async fn serve_until<S, Req, Sd>(self, new_service: S, shutdown_signal: Sd)
    where
        S: NewService<Request = Req, Response = Response> + Send + Sync + 'static,
        Sd: Future<Output = ()> + Sync + Send + Unpin + 'static,
        S::Request: From<Req> + From<rtu::RequestAdu>,
        S::Response: Into<Response>,
        S::Error: Into<Error>,
        S::Instance: Send + Sync + 'static,
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
async fn process<S, Req>(
    mut framed: Framed<SerialStream, codec::rtu::ServerCodec>,
    service: S,
) -> Result<(), Error>
where
    S: Service<Request = Req, Response = Response> + Send + Sync + 'static,
    S::Error: Into<Error>,
    S::Request: From<rtu::RequestAdu>,
{
    loop {
        let request = match framed.next().await {
            // Stream is exhausted
            None => break,
            Some(request) => request,
        }?;

        let hdr = request.hdr;
        let response = service.call(request.into()).await.map_err(Into::into)?;
        framed
            .send(rtu::ResponseAdu {
                hdr,
                pdu: response.into(),
            })
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::Service;

    use futures::future;

    #[tokio::test]
    async fn service_wrapper() {
        #[derive(Clone)]
        struct DummyService {
            response: Response,
        }

        impl Service for DummyService {
            type Request = Request;
            type Response = Response;
            type Error = Error;
            type Future = future::Ready<Result<Self::Response, Self::Error>>;

            fn call(&self, _: Self::Request) -> Self::Future {
                future::ready(Ok(self.response.clone()))
            }
        }

        let service = DummyService {
            response: Response::ReadInputRegisters(vec![0x33]),
        };

        let pdu = Request::ReadInputRegisters(0, 1);
        let rsp_adu = service.call(pdu).await.unwrap();

        assert_eq!(rsp_adu, service.response);
    }

    #[tokio::test]
    async fn slave_service_wrapper() {
        #[derive(Clone)]
        struct DummyService {
            response: Response,
        }

        impl Service for DummyService {
            type Request = SlaveRequest;
            type Response = Response;
            type Error = Error;
            type Future = future::Ready<Result<Self::Response, Self::Error>>;

            fn call(&self, _: Self::Request) -> Self::Future {
                future::ready(Ok(self.response.clone()))
            }
        }

        let service = DummyService {
            response: Response::ReadInputRegisters(vec![0x33]),
        };

        let pdu = Request::ReadInputRegisters(0, 1);
        let rsp_adu = service.call(pdu).await.unwrap();

        assert_eq!(rsp_adu, service.response);
    }
}
