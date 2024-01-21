// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Execute this test only if `rtu-over-tcp-server` feature is selected.

#![cfg(feature = "rtu-over-tcp-server")]

mod exception;

use std::{net::SocketAddr, time::Duration};

use exception::check_client_context;
use tokio::net::TcpListener;
use tokio_modbus::{
    server::rtu_over_tcp::{accept_tcp_connection, Server},
    Slave,
};

use crate::exception::TestService;

#[tokio::test]
async fn all_exceptions() -> Result<(), Box<dyn std::error::Error>> {
    let socket_addr = "127.0.0.1:5503".parse().unwrap();

    tokio::select! {
        _ = server_context(socket_addr) => unreachable!(),
        _ = client_context(socket_addr) => (),
    }

    Ok(())
}

async fn server_context(socket_addr: SocketAddr) -> anyhow::Result<()> {
    println!("Starting up server on {socket_addr}");
    let listener = TcpListener::bind(socket_addr).await?;
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

// TODO: Update the `assert_eq` with a check on Exception once Client trait can return Exception
async fn client_context(socket_addr: SocketAddr) {
    // Give the server some time for starting up
    tokio::time::sleep(Duration::from_millis(100)).await;

    let transport = tokio::net::TcpStream::connect(socket_addr).await.unwrap();
    let ctx = tokio_modbus::prelude::rtu::attach_slave(transport, Slave(1));

    check_client_context(ctx).await;
}
