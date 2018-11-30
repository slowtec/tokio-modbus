use super::*;

use crate::frame::*;
use crate::proto;

use std::io::Error;
use std::net::SocketAddr;
use tokio_proto::TcpServer;
use tokio_service::NewService;

/// A multithreaded Modbus TCP server.
pub struct Server {
    socket_addr: SocketAddr,
}

impl Server {
    /// Create a new Modbus TCP server instance.
    pub fn new(socket_addr: SocketAddr) -> Self {
        Server { socket_addr }
    }

    pub fn serve<S>(&self, service: S)
    where
        S: NewService + Send + Sync + 'static,
        S::Request: From<Request>,
        S::Response: Into<Response>,
        S::Error: Into<Error>,
        S::Instance: Send + Sync + 'static,
    {
        TcpServer::new(proto::tcp::Proto, self.socket_addr)
            .serve(move || Ok(ServiceWrapper::new(service.new_service()?)));
    }
}
