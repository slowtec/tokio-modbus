use super::Context;

use crate::service;

use futures::prelude::*;
use std::io::Error;
use tokio_core::reactor::Handle;
use tokio_serial::Serial;

pub fn connect(
    serial: Serial,
    address: u8,
    handle: &Handle,
) -> impl Future<Item = Context, Error = Error> {
    service::rtu::Client::connect(serial, address, handle).map(|service| Context {
        service: Box::new(service),
    })
}
