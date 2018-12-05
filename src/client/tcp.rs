use super::Connection;

use crate::service;

use futures::prelude::*;
use std::io::Error;
use std::net::SocketAddr;
use tokio_core::reactor::Handle;

pub fn connect(
    socket_addr: SocketAddr,
    handle: &Handle,
) -> impl Future<Item = Connection, Error = Error> {
    service::tcp::Client::connect(&socket_addr, handle).map(|service| Connection {
        service: Box::new(service),
    })
}
