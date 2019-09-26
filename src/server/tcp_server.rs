use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use net2;

use futures::{select, future::Future};
use tokio::net::{TcpStream, TcpListener};
use crate::{NewService, Service};
use crate::codec;
use log::{error, trace};
use futures_util::stream::StreamExt;
use futures_util::sink::SinkExt;
use futures_util::future::FutureExt;
use tokio_util::codec::Framed;

// TODO: Add more options, e.g.:
// - max concurrent requests
// - request timeout
// - read timeout
// - write timeout
// - max idle time
// - max lifetime

/// A builder for TCP servers.
///
/// Setting up a server needs, at minimum:
///
/// - A server protocol implementation
/// - An address
/// - A service to provide
///
/// In addition to those basics, the builder provides some additional
/// configuration, which is expected to grow over time.
///
/// See the crate docs for an example.
#[derive(Debug)]
pub struct TcpServer {
    threads: usize,
    addr: SocketAddr,
}

impl TcpServer {
    /// Starts building a server for the given protocol and address, with
    /// default configuration.
    ///
    pub fn new(addr: SocketAddr) -> TcpServer {
        TcpServer {
            threads: 1,
            addr,
        }
    }

    /// Set the address for the server.
    pub fn addr(&mut self, addr: SocketAddr) {
        self.addr = addr;
    }

    /// Set the number of threads running simultaneous event loops (Unix only).
    pub fn threads(&mut self, threads: usize) {
        assert!(threads > 0);
        if cfg!(unix) {
            self.threads = threads;
        }
    }

    // /// Start up the server, providing the given service on it.
    // ///
    // /// This method will block the current thread until the server is shut down.
    pub fn serve<S, Sd>(&self, new_service: S) where
        S: NewService<Request = crate::frame::Request, Response = crate::frame::Response> + Send + Sync + 'static,
        S::Instance: 'static + Send + Sync
    {
        self.serve_until(new_service, futures::future::pending())
    }

    /// Start up the server, providing the given service on it.
    ///
    /// This method will block the current thread until the server is shut down or until the given future, `shutdown_signal` resolves.
    pub fn serve_until<S, Sd>(&self, new_service: S, shutdown_signal: Sd) where
        S: NewService<Request = crate::frame::Request, Response = crate::frame::Response> + Send + Sync + 'static,
        Sd: Future<Output = ()> + Send + Sync + Unpin + 'static,
        S::Instance: 'static + Send + Sync
    {
        self.with_handle(new_service, shutdown_signal)
    }

    /// Start up the server, providing the given service on it, and providing
    /// access to the event loop handle.
    ///
    /// The `new_service` argument is a closure that is given an event loop
    /// handle, and produces a value implementing `NewService`. That value is in
    /// turn used to make a new service instance for each incoming connection.
    ///
    /// This method will block the current thread until the server is shut down.
    pub fn with_handle<S, Sd>(&self, new_service: S, shutdown_signal: Sd) where
        S: NewService<Request = crate::frame::Request, Response = crate::frame::Response> + Send + Sync + 'static,
        Sd: Future<Output = ()> + Send + Sync + Unpin + 'static,
        S::Instance: 'static + Send + Sync
    {
        // let proto = self.proto.clone();
        // let new_service = Arc::new(new_service);
        let addr = self.addr;
        let workers = self.threads;

        serve_until(addr, workers, new_service, shutdown_signal);
    }
}

fn serve_until<S, Sd>(addr: SocketAddr, workers: usize, new_service: S, shutdown_signal: Sd)
    where
        S: NewService<Request = crate::frame::Request, Response = crate::frame::Response> + Send + Sync + 'static,
        S::Instance: 'static + Send + Sync,
        Sd: Future<Output = ()> + Unpin + Send + Sync + 'static
{
    let mut rt = tokio::runtime::Runtime::new().unwrap();

    let new_service = Arc::new(new_service);

    let server = async {
        let mut listener = listener(&addr, workers).unwrap();

        loop {
            let (stream, _) = listener.accept().await?;
            let framed = Framed::new(stream, codec::tcp::ServerCodec::default());

            let new_service = new_service.clone();
            tokio::spawn(Box::pin(async move {
                let service = new_service.new_service().unwrap();
                let future = process(framed, service);

                future.await.unwrap();
            }));
        }

        // the only way found to specify the "task" future error type
        #[allow(unreachable_code)]
        Result::<(), std::io::Error>::Ok(())
    };

    let mut server = Box::pin(server.fuse());
    let mut shutdown_signal = shutdown_signal.fuse();

    let task = async {
        select! {
            res = server => match res {
                Err(e) => error!("error: {}", e),
                _ => {}
            },
            _ = shutdown_signal => { trace!("Shutdown signal received") }
        }
    };

    rt.block_on(task);
}

async fn process<S>(framed: Framed<TcpStream, codec::tcp::ServerCodec>, service: S) -> Result<(), std::io::Error>
    where
        S: Service<Request = crate::frame::Request, Response = crate::frame::Response> + Send + Sync + 'static,
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
        let response = service.call(request.pdu.0);

        framed.send(crate::frame::tcp::ResponseAdu {
            hdr,
            pdu: response.into()
        }).await?;
    }
    Ok(())
}

fn listener(addr: &SocketAddr,
            workers: usize) -> io::Result<TcpListener> {
    let listener = match *addr {
        SocketAddr::V4(_) => net2::TcpBuilder::new_v4()?,
        SocketAddr::V6(_) => net2::TcpBuilder::new_v6()?,
    };
    configure_tcp(workers, &listener)?;
    listener.reuse_address(true)?;
    listener.bind(addr)?;
    listener.listen(1024).and_then(TcpListener::from_std)
}

#[cfg(unix)]
fn configure_tcp(workers: usize, tcp: &net2::TcpBuilder) -> io::Result<()> {
    use net2::unix::*;

    if workers > 1 {
        tcp.reuse_port(true)?;
    }
    
    Ok(())
}

#[cfg(windows)]
fn configure_tcp(_workers: usize, _tcp: &net2::TcpBuilder) -> io::Result<()> {
    Ok(())
}
