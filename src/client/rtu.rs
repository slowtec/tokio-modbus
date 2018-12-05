use super::Connection;

use crate::service;

use futures::prelude::*;
use std::io::Error;
use tokio_core::reactor::Handle;
use tokio_serial::Serial;

pub fn connect(
    serial: Serial,
    address: u8,
    handle: &Handle,
) -> impl Future<Item = Connection, Error = Error> {
    service::rtu::Client::connect(serial, address, handle).map(|service| Connection {
        service: Box::new(service),
    })
}
