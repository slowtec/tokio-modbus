// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Execute this test only if `rtu-server` feature is selected.

#![cfg(feature = "rtu-server")]

mod exception;

use std::{thread, time::Duration};

use exception::check_client_context;
use tokio_modbus::{
    client::{self},
    server::rtu::Server,
};
use tokio_serial::SerialPortBuilder;

use crate::exception::TestService;

#[tokio::test]
#[ignore = "we need to mock a serial port to test this"]
async fn all_exceptions() -> Result<(), Box<dyn std::error::Error>> {
    let builder = tokio_serial::new("/dev/ttyUSB0", 19200);

    tokio::select! {
        _ = server_context(&builder) => unreachable!(),
        _ = client_context(&builder) => (),
    }

    Ok(())
}

async fn server_context(builder: &SerialPortBuilder) -> anyhow::Result<()> {
    let server_serial = tokio_serial::SerialStream::open(builder).unwrap();

    let _server = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let server = Server::new(server_serial);
        let service = TestService {};
        rt.block_on(async {
            if let Err(err) = server.serve_forever(service).await {
                eprintln!("{err}");
            }
        });
    });

    Ok(())
}

// TODO: Update the `assert_eq` with a check on Exception once Client trait can return Exception
async fn client_context(builder: &SerialPortBuilder) {
    // Give the server some time for starting up
    tokio::time::sleep(Duration::from_millis(100)).await;
    let client_serial = tokio_serial::SerialStream::open(builder).unwrap();

    let ctx = client::rtu::attach(client_serial);

    check_client_context(ctx).await;
}
