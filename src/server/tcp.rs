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
    net::{TcpListener, TcpStream},
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

/// Accept unencrypted TCP connections.
pub fn accept_tcp_connection<S, NewService>(
    stream: TcpStream,
    socket_addr: SocketAddr,
    new_service: NewService,
) -> io::Result<Option<(S, TcpStream)>>
where
    S: Service + Send + Sync + 'static,
    S::Request: From<RequestAdu<'static>> + Send,
    S::Response: Into<OptionalResponsePdu> + Send,
    S::Error: Into<io::Error>,
    NewService: Fn(SocketAddr) -> io::Result<Option<S>>,
{
    let service = new_service(socket_addr)?;
    Ok(service.map(|service| (service, stream)))
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

    /// Listens for incoming connections and starts a Modbus TCP server task for
    /// each connection.
    ///
    /// `OnConnected` is responsible for creating both the service and the
    /// transport layer for the underlying TCP stream. If `OnConnected` returns
    /// with `Err` then listening stops and [`Self::serve()`] returns with an error.
    /// If `OnConnected` returns `Ok(None)` then the connection is rejected
    /// but [`Self::serve()`] continues listening for new connections.
    pub async fn serve<S, T, F, OnConnected, OnProcessError>(
        &self,
        on_connected: &OnConnected,
        on_process_error: OnProcessError,
    ) -> io::Result<()>
    where
        S: Service + Send + Sync + 'static,
        S::Request: From<RequestAdu<'static>> + Send,
        S::Response: Into<OptionalResponsePdu> + Send,
        S::Error: Into<io::Error>,
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        OnConnected: Fn(TcpStream, SocketAddr) -> F,
        F: Future<Output = io::Result<Option<(S, T)>>>,
        OnProcessError: FnOnce(io::Error) + Clone + Send + 'static,
    {
        loop {
            let (stream, socket_addr) = self.listener.accept().await?;
            log::debug!("Accepted connection from {socket_addr}");

            let Some((service, transport)) = on_connected(stream, socket_addr).await? else {
                log::debug!("No service for connection from {socket_addr}");
                continue;
            };
            let on_process_error = on_process_error.clone();

            let framed = Framed::new(transport, ServerCodec::default());

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
    pub async fn serve_until<S, T, F, X, OnConnected, OnProcessError>(
        self,
        on_connected: &OnConnected,
        on_process_error: OnProcessError,
        abort_signal: X,
    ) -> io::Result<Terminated>
    where
        S: Service + Send + Sync + 'static,
        S::Request: From<RequestAdu<'static>> + Send,
        S::Response: Into<OptionalResponsePdu> + Send,
        S::Error: Into<io::Error>,
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        X: Future<Output = ()> + Sync + Send + Unpin + 'static,
        OnConnected: Fn(TcpStream, SocketAddr) -> F,
        F: Future<Output = io::Result<Option<(S, T)>>>,
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

/// The request-response loop spawned by [`serve_until`] for each client
async fn process<S, T, Req, Res>(mut framed: Framed<T, ServerCodec>, service: S) -> io::Result<()>
where
    S: Service<Request = Req, Response = Res> + Send + Sync + 'static,
    S::Request: From<RequestAdu<'static>> + Send,
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
            .into()
        else {
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

    use std::sync::Arc;

    use futures::future;

    #[tokio::test]
    async fn delegate_service_through_deref_for_server() {
        #[derive(Clone)]
        struct DummyService {
            response: Response,
        }

        impl Service for DummyService {
            type Request = Request<'static>;
            type Response = Response;
            type Error = io::Error;
            type Future = future::Ready<Result<Self::Response, Self::Error>>;

            fn call(&self, _: Self::Request) -> Self::Future {
                future::ready(Ok(self.response.clone()))
            }
        }

        let service = Arc::new(DummyService {
            response: Response::ReadInputRegisters(vec![0x33]),
        });
        let svc = |_socket_addr| Ok(Some(Arc::clone(&service)));
        let on_connected =
            |stream, socket_addr| async move { accept_tcp_connection(stream, socket_addr, svc) };

        // bind 0 to let the OS pick a random port
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let listener = TcpListener::bind(addr).await.unwrap();
        let server = Server::new(listener);

        // passes type-check is the goal here
        // added `mem::drop` to satisfy `must_use` compiler warnings
        std::mem::drop(server.serve(&on_connected, |_err| {}));
    }

    #[tokio::test]
    async fn service_wrapper() {
        #[derive(Clone)]
        struct DummyService {
            response: Response,
        }

        impl Service for DummyService {
            type Request = Request<'static>;
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
