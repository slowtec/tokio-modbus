// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus TCP server skeleton

use crate::{
    codec,
    frame::*,
    server::service::{NewService, Service},
};

use futures::{self, Future};
use futures_util::{future::FutureExt as _, sink::SinkExt as _, stream::StreamExt as _};
use socket2::{Domain, Socket, Type};
use std::{
    io::{self, BufReader, Error},
    net::SocketAddr,
    sync::Arc,
};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;

use rustls_pemfile::{certs, ec_private_keys};
use std::convert::From;
use std::fs::File;
use std::path::Path;
use tokio_rustls::rustls::{self, Certificate, PrivateKey};
use tokio_rustls::TlsAcceptor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    socket_addr: SocketAddr,
}

fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
    ec_private_keys(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))
        .map(|mut keys| keys.drain(..).map(PrivateKey).collect())
}

impl Server {
    /// Set the address for the server (mandatory).
    #[must_use]
    pub fn new(socket_addr: SocketAddr) -> Self {
        Self { socket_addr }
    }

    /// Start an async Modbus TCP server task.
    pub async fn serve<S, Req, Res>(&self, service: S) -> Result<(), std::io::Error>
    where
        S: NewService<Request = Req, Response = Res> + Send + Sync + 'static,
        Req: From<tcp::RequestAdu> + Send,
        Res: Into<OptionalResponsePdu> + Send,
        S::Instance: Send + Sync + 'static,
        S::Error: Into<Error>,
    {
        let service = Arc::new(service);

        let cert_path = Path::new("./pki/server.pem");
        let key_path = Path::new("./pki/server.key");
        let certs = load_certs(cert_path)?;
        let mut keys = load_keys(key_path)?;
        //println!("{:?}", keys);
        let config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, keys.remove(0))
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        let acceptor = TlsAcceptor::from(Arc::new(config));

        let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
        socket.set_nodelay(true)?;
        socket.bind(&self.socket_addr.into())?;
        socket.listen(1024)?;
        let listener = TcpListener::from_std(socket.into())?;

        loop {
            //let (stream, _) = listener.accept().await?;
            let (stream, _) = listener.accept().await?;
            let acceptor = acceptor.clone();

            let stream = acceptor.accept(stream).await?;
            let framed = Framed::new(stream, codec::tcp::ServerCodec::default());
            let new_service = service.clone();

            tokio::spawn(Box::pin(async move {
                let service = new_service.new_service().unwrap();
                if let Err(err) = process(framed, service).await {
                    eprintln!("TLS Server: {err:?}");
                }
            }));
        }
    }

    /// Start a Modbus TCP server that blocks the current thread until a shutdown is requested
    pub fn serve_until<S, Req, Res, Sd>(self, service: S, shutdown_signal: Sd)
    where
        S: NewService<Request = Req, Response = Res> + Send + Sync + 'static,
        Sd: Future<Output = ()> + Sync + Send + Unpin + 'static,
        Req: From<tcp::RequestAdu> + Send,
        Res: Into<OptionalResponsePdu> + Send,
        S::Instance: Send + Sync + 'static,
        S::Error: Into<Error>,
    {
        let shutdown_signal = shutdown_signal.fuse();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .build()
            .unwrap();

        rt.block_on(async {
            tokio::select! {
                res = self.serve(service) => if let Err(e) = res { log::error!("Error: {}", e) },
                _ = shutdown_signal => log::trace!("Shutdown signal received")
            }
        })
    }

    pub fn serve_forever<S, Req, Res>(self, service: S)
    where
        S: NewService<Request = Req, Response = Res> + Send + Sync + 'static,
        Req: From<tcp::RequestAdu> + Send,
        Res: Into<OptionalResponsePdu> + Send,
        S::Instance: Send + Sync + 'static,
        S::Error: Into<Error>,
    {
        self.serve_until(service, futures::future::pending())
    }
}

/// The request-response loop spawned by serve_until for each client
async fn process<S, Req, Res>(
    framed: Framed<tokio_rustls::server::TlsStream<TcpStream>, codec::tcp::ServerCodec>,
    service: S,
) -> io::Result<()>
where
    S: Service<Request = Req, Response = Res> + Send + Sync + 'static,
    S::Request: From<tcp::RequestAdu> + Send,
    S::Response: Into<OptionalResponsePdu> + Send,
    S::Error: Into<Error>,
{
    let mut framed = framed;

    loop {
        let request = framed.next().await;

        // tcp socket closed
        if request.is_none() {
            break;
        }

        let request = request.unwrap()?;
        let hdr = request.hdr;
        let response: OptionalResponsePdu = service
            .call(request.into())
            .await
            .map_err(Into::into)?
            .into();

        match response.0 {
            Some(pdu) => {
                framed.send(tcp::ResponseAdu { hdr, pdu }).await?;
            }
            None => {
                log::debug!("No response for request {hdr:?}");
            }
        }
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
    configure_tls(workers, &listener)?;
    listener.reuse_address()?;
    listener.set_nodelay(true)?;
    listener.bind(&addr.into())?;
    listener.listen(1024)?;
    TcpListener::from_std(listener.into())
}

#[cfg(unix)]
#[allow(unused)]
fn configure_tls(workers: usize, tls: &Socket) -> io::Result<()> {
    if workers > 1 {
        tls.reuse_port()?;
    }
    Ok(())
}

#[cfg(windows)]
#[allow(unused)]
fn configure_tls(_workers: usize, _tcp: &Socket) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {

    mod server {

        #[test]
        fn decode_header_fragment() {
            let cert_path = crate::server::tls::Path::new("./pki/server.pem");
            let key_path = crate::server::tls::Path::new("./pki/server.key");
            let certs = crate::server::tls::load_certs(cert_path).unwrap();
            let keys = crate::server::tls::load_keys(key_path).unwrap();
            assert_eq!(certs.len(), 1);
            assert_eq!(keys.len(), 1);
        }
    }
}
