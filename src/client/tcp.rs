use super::Context;

use crate::service;

use futures::prelude::*;
use std::io::Error;
use std::net::SocketAddr;
use tokio_core::reactor::Handle;

pub fn connect(
    handle: &Handle,
    socket_addr: SocketAddr,
) -> impl Future<Item = Context, Error = Error> {
    service::tcp::Client::connect(handle, socket_addr).map(|service| Context {
        service: Box::new(service),
    })
}
