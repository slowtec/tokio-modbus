// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test repeated writes to holding registers with the TCP client and TCP server.
//! Test for [#301 memory leak in TCP client](https://github.com/slowtec/tokio-modbus/issues/301).

#![cfg(feature = "tcp-server")]

#[allow(unused)]
mod exception;

use std::net::SocketAddrV4;
use std::{net::SocketAddr, time::Duration};

use tokio::net::TcpListener;
use tokio_modbus::prelude::*;
use tokio_modbus::{
    client,
    server::tcp::{accept_tcp_connection, Server},
};

use crate::exception::TestService;

#[tokio::test]
async fn tcp_issue301_write_test() -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr: SocketAddrV4 = "127.0.0.1:0".parse().unwrap();
    let listener = TcpListener::bind(bind_addr).await?;
    let server_addr = listener.local_addr()?;

    tokio::select! {
        _ = tokio::time::sleep(Duration::from_millis(10000)) => panic!("timeout - perhaps we have a lockup or a thread was killed"),
        r = server_context(listener) => r?,
        r = client_context(server_addr) => r?,
    }

    Ok(())
}

async fn server_context(listener: TcpListener) -> anyhow::Result<()> {
    println!("Starting up server on {:? }", listener.local_addr()?);
    let server = Server::new(listener);
    let new_service = |_socket_addr| Ok(Some(TestService {}));
    let on_connected = |stream, socket_addr| async move {
        accept_tcp_connection(stream, socket_addr, new_service)
    };
    let on_process_error = |err| {
        eprintln!("{err}");
    };
    server.serve(&on_connected, on_process_error).await?;
    Ok(())
}

async fn client_context(socket_addr: SocketAddr) -> anyhow::Result<()> {
    // Give the server some time for starting up
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut ctx = client::tcp::connect(socket_addr).await?;

    let max_iterations = 1000;
    let mut iterations = 1;
    loop {
        println!("iteration {}", iterations);
        // In this case, the TestService always returns an exception, we'll ignore that.
        // We are only interested here in the TCP client side constructing the request.
        _ = ctx.write_multiple_registers(0x1000, &[1]).await?;
        _ = ctx.write_multiple_registers(0x1000, &[1, 2]).await?;

        if iterations == max_iterations {
            return Ok(());
        }

        iterations += 1;
    }
}
