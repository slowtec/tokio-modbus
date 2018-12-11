use super::Context;

use crate::service;

use futures::prelude::*;
use std::io::Error;
use tokio_core::reactor::Handle;
use tokio_serial::Serial;

pub fn connect(
    handle: &Handle,
    serial: Serial,
    slave: u8,
) -> impl Future<Item = Context, Error = Error> {
    service::rtu::Client::bind(handle, serial, slave).map(|service| Context {
        service: Box::new(service),
    })
}
