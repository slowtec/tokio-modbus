// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus TCP server skeleton

use std::{io, net::SocketAddr};

use async_trait::async_trait;
use futures::{self, Future};
use futures_util::{future::FutureExt as _, sink::SinkExt as _, stream::StreamExt as _};
use socket2::{Domain, Socket, Type};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
};
use tokio_util::codec::Framed;

use crate::{
    codec::tcp::ServerCodec,
    frame::{
        tcp::{RequestAdu, ResponseAdu},
        OptionalResponsePdu,
    },
    server::service::Service,
};

use super::Terminated;

#[async_trait]
pub trait BindSocket {
    type Error;

    async fn bind_socket(addr: SocketAddr) -> Result<Socket, Self::Error>;
}

#[derive(Debug)]
pub struct Server {
    listener: TcpListener,
}

impl Server {
    /// Attach the Modbus server to a TCP socket server.
    #[must_use]
    pub fn new(listener: TcpListener) -> Self {
        Self { listener }
    }

    /// Start a Modbus TCP server task.
    pub async fn serve<S, OnConnected, OnProcessError>(
        &self,
        on_connected: &OnConnected,
        on_process_error: OnProcessError,
    ) -> io::Result<()>
    where
        S: Service + Send + Sync + 'static,
        S::Request: From<RequestAdu> + Send,
        S::Response: Into<OptionalResponsePdu> + Send,
        S::Error: Into<io::Error>,
        OnConnected: Fn(SocketAddr) -> Option<S>,
        OnProcessError: FnOnce(io::Error) + Clone + Send + 'static,
    {
        loop {
            let (stream, socket_addr) = self.listener.accept().await?;
            log::debug!("Accepted connection from {socket_addr}");

            let Some(service) = on_connected(socket_addr) else {
                log::debug!("No service for connection from {socket_addr}");
                continue;
            };
            let on_process_error = on_process_error.clone();
            let framed = Framed::new(stream, ServerCodec::default());

            tokio::spawn(async move {
                log::debug!("Processing requests from {socket_addr}");
                if let Err(err) = process(framed, service).await {
                    on_process_error(err);
                }
            });
        }
    }

    /// Start an abortable Modbus TCP server task.
    ///
    /// Warning: Request processing is not scoped and could be aborted at any internal await point!
    /// See also: <https://rust-lang.github.io/wg-async/vision/roadmap/scopes.html#cancellation>
    pub async fn serve_until<S, X, OnConnected, OnProcessError>(
        self,
        on_connected: &OnConnected,
        on_process_error: OnProcessError,
        abort_signal: X,
    ) -> io::Result<Terminated>
    where
        S: Service + Send + Sync + 'static,
        S::Request: From<RequestAdu> + Send,
        S::Response: Into<OptionalResponsePdu> + Send,
        S::Error: Into<io::Error>,
        X: Future<Output = ()> + Sync + Send + Unpin + 'static,
        OnConnected: Fn(SocketAddr) -> Option<S>,
        OnProcessError: FnOnce(io::Error) + Clone + Send + 'static,
    {
        let abort_signal = abort_signal.fuse();
        tokio::select! {
            res = self.serve(on_connected, on_process_error) => {
                res.map(|()| Terminated::Finished)
            },
            () = abort_signal => {
                Ok(Terminated::Aborted)
            }
        }
    }
}

/// The request-response loop spawned by serve_until for each client
async fn process<S, T, Req, Res>(mut framed: Framed<T, ServerCodec>, service: S) -> io::Result<()>
where
    S: Service<Request = Req, Response = Res> + Send + Sync + 'static,
    S::Request: From<RequestAdu> + Send,
    S::Response: Into<OptionalResponsePdu> + Send,
    S::Error: Into<io::Error>,
    T: AsyncRead + AsyncWrite + Unpin,
{
    loop {
        let Some(request) = framed.next().await.transpose()? else {
            log::debug!("TCP socket has been closed");
            break;
        };

        let hdr = request.hdr;
        let OptionalResponsePdu(Some(response_pdu)) = service
            .call(request.into())
            .await
            .map_err(Into::into)?
            .into() else {
                log::trace!("Sending no response for request {hdr:?}");
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

/// Start TCP listener - configure and open TCP socket
#[allow(unused)]
fn listener(addr: SocketAddr, workers: usize) -> io::Result<TcpListener> {
    let listener = match addr {
        SocketAddr::V4(_) => Socket::new(Domain::IPV4, Type::STREAM, None)?,
        SocketAddr::V6(_) => Socket::new(Domain::IPV6, Type::STREAM, None)?,
    };
    configure_tcp(workers, &listener)?;
    listener.reuse_address()?;
    listener.bind(&addr.into())?;
    listener.listen(1024)?;
    TcpListener::from_std(listener.into())
}

#[cfg(unix)]
#[allow(unused)]
fn configure_tcp(workers: usize, tcp: &Socket) -> io::Result<()> {
    if workers > 1 {
        tcp.reuse_port()?;
    }
    Ok(())
}

#[cfg(windows)]
#[allow(unused)]
fn configure_tcp(_workers: usize, _tcp: &Socket) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{prelude::*, server::Service};

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
            type Error = io::Error;
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
