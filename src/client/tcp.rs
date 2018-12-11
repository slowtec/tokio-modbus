use super::Context;

use crate::service;

use futures::prelude::*;
use std::io::Error;
use std::net::SocketAddr;
use tokio_core::reactor::Handle;

pub fn connect(
    socket_addr: SocketAddr,
    handle: &Handle,
) -> impl Future<Item = Context, Error = Error> {
    service::tcp::Client::connect(&socket_addr, handle).map(|service| Context {
        service: Box::new(service),
    })
}
