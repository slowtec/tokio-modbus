// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test to demonstrate that RTU over TCP server properly disconnects clients when cancelled.
//! Test for [#329 Cancelling or otherwise shutting down the TCP server does not disconnect clients](https://github.com/slowtec/tokio-modbus/issues/329).
//! Test for [#333 serve_until is not handing current existing clients](https://github.com/slowtec/tokio-modbus/issues/333).

#![cfg(feature = "rtu-over-tcp-server")]

#[allow(unused)]
mod exception;

use std::{net::SocketAddr, time::Duration};

use futures::future::FutureExt;
use tokio::{
    net::TcpListener,
    sync::oneshot::{channel, Receiver, Sender},
};
use tokio_modbus::{
    prelude::*,
    server::rtu_over_tcp::{accept_tcp_connection, Server},
    server::Terminated,
};

use crate::exception::TestService;

#[tokio::test]
async fn test_server_shutdown_disconnects_clients() {
    let socket_addr = "127.0.0.1:5502".parse().unwrap();
    let (sender, receiver) = channel();
    let server_task = tokio::spawn(server_context(socket_addr, receiver));
    let client_task = tokio::spawn(client_context(socket_addr, sender));
    assert!(matches!(
        server_task.await.unwrap(),
        Ok(Terminated::Aborted)
    ));
    client_task.await.unwrap();
}

async fn server_context(
    socket_addr: SocketAddr,
    receiver: Receiver<()>,
) -> std::io::Result<Terminated> {
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
    let abort_signal = receiver.map(|_| ());
    server
        .serve_until(&on_connected, on_process_error, abort_signal)
        .await
}

async fn client_context(socket_addr: SocketAddr, sender: Sender<()>) {
    // Give the server some time for starting up
    tokio::time::sleep(Duration::from_millis(100)).await;
    // Connect to server
    let transport = tokio::net::TcpStream::connect(socket_addr).await.unwrap();
    let mut ctx = tokio_modbus::prelude::rtu::attach_slave(transport, Slave(1));
    // Check that a request receives a response
    assert!(ctx.read_input_registers(0, 1).await.is_ok());
    // Stop the server
    sender.send(()).unwrap();
    // Give the server some time for winding down
    tokio::time::sleep(Duration::from_millis(100)).await;
    // Check that a request does not receive a response
    assert!(ctx.read_input_registers(0, 1).await.is_err());
}
