// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus TCP server skeleton

use std::{future::Future, io, net::SocketAddr};

use async_trait::async_trait;
use futures_util::{FutureExt as _, SinkExt as _, StreamExt as _};
use socket2::{Domain, Socket, Type};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
    task::JoinSet,
};
use tokio_util::codec::Framed;

use crate::{
    codec::tcp::ServerCodec,
    frame::{
        tcp::{RequestAdu, ResponseAdu},
        ExceptionResponse, OptionalResponsePdu, RequestPdu,
    },
};

use super::{Service, Terminated};

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
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        OnConnected: Fn(TcpStream, SocketAddr) -> F,
        F: Future<Output = io::Result<Option<(S, T)>>>,
        OnProcessError: FnOnce(io::Error) + Clone + Send + 'static,
    {
        let mut join_set = JoinSet::new();
        loop {
            let (stream, socket_addr) = self.listener.accept().await?;
            log::debug!("Accepted connection from {socket_addr}");

            let Some((service, transport)) = on_connected(stream, socket_addr).await? else {
                log::debug!("No service for connection from {socket_addr}");
                continue;
            };
            let on_process_error = on_process_error.clone();

            let framed = Framed::new(transport, ServerCodec::default());

            join_set.spawn(async move {
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
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        X: Future<Output = ()> + Sync + Send + Unpin + 'static,
        OnConnected: Fn(TcpStream, SocketAddr) -> F,
        F: Future<Output = io::Result<Option<(S, T)>>>,
        OnProcessError: FnOnce(io::Error) + Clone + Send + 'static,
    {
        let mut join_set = JoinSet::new();
        let abort_signal = abort_signal.fuse();
        
        let serve_result = tokio::select! {
            res = self.serve_with_joinset(&mut join_set, on_connected, on_process_error) => {
                // Server finished naturally (should never happen in practice)
                res.map(|()| Terminated::Finished)
            },
            () = abort_signal => {
                log::debug!("Abort signal received, shutting down server and active connections");
                // Abort all connection tasks
                join_set.abort_all();
                // Wait for all tasks to be aborted
                while join_set.join_next().await.is_some() {
                    // Join all tasks (they should complete quickly due to abort)
                }
                Ok(Terminated::Aborted)
            }
        };
        
        serve_result
    }

    /// Internal serve method that accepts a JoinSet for connection tracking
    async fn serve_with_joinset<S, T, F, OnConnected, OnProcessError>(
        &self,
        join_set: &mut JoinSet<()>,
        on_connected: &OnConnected,
        on_process_error: OnProcessError,
    ) -> io::Result<()>
    where
        S: Service + Send + Sync + 'static,
        S::Request: From<RequestAdu<'static>> + Send,
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

            join_set.spawn(async move {
                log::debug!("Processing requests from {socket_addr}");
                if let Err(err) = process(framed, service).await {
                    on_process_error(err);
                }
                log::debug!("Connection from {socket_addr} closed");
            });
        }
    }
}

/// The request-response loop spawned by [`serve_until`] for each client
async fn process<S, T>(mut framed: Framed<T, ServerCodec>, service: S) -> io::Result<()>
where
    S: Service + Send + Sync + 'static,
    S::Request: From<RequestAdu<'static>> + Send,
    T: AsyncRead + AsyncWrite + Unpin,
{
    loop {
        let Some(request_adu) = framed.next().await.transpose().inspect_err(|err| {
            log::debug!("Failed to receive and decode request ADU: {err}");
        })?
        else {
            log::debug!("TCP socket has been closed");
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

    use std::{future, sync::Arc};

    #[tokio::test]
    async fn delegate_service_through_deref_for_server() {
        #[derive(Clone)]
        struct DummyService {
            response: Response,
        }

        impl Service for DummyService {
            type Request = Request<'static>;
            type Response = Response;
            type Exception = ExceptionCode;
            type Future = future::Ready<Result<Self::Response, Self::Exception>>;

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
            type Exception = ExceptionCode;
            type Future = future::Ready<Result<Self::Response, ExceptionCode>>;

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
