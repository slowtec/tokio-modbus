// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test to demonstrate that TCP server properly disconnects clients when cancelled

use std::{future, net::SocketAddr, time::Duration};

use tokio::{net::TcpListener, time::timeout};

use tokio_modbus::{
    prelude::*,
    server::tcp::{accept_tcp_connection, Server},
};

#[derive(Clone)]
struct TestService;

impl tokio_modbus::server::Service for TestService {
    type Request = Request<'static>;
    type Response = Response;
    type Exception = ExceptionCode;
    type Future = future::Ready<Result<Self::Response, Self::Exception>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let res = match req {
            Request::ReadInputRegisters(_addr, cnt) => {
                Ok(Response::ReadInputRegisters(vec![0; cnt as usize]))
            }
            _ => Err(ExceptionCode::IllegalFunction),
        };
        future::ready(res)
    }
}

#[tokio::test]
async fn test_server_shutdown_disconnects_clients() {
    let socket_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = TcpListener::bind(socket_addr).await.unwrap();
    let server_addr = listener.local_addr().unwrap();
    
    let server = Server::new(listener);
    // Create an abort signal
    let (abort_tx, abort_rx) = tokio::sync::oneshot::channel::<()>();
    
    // Start the server with abort signal
    let server_task = tokio::spawn(async move {
        let new_service = |_socket_addr| Ok(Some(TestService));
        let on_connected = |stream, socket_addr| async move {
            accept_tcp_connection(stream, socket_addr, new_service)
        };
        let on_process_error = |err| {
            eprintln!("Server error: {err}");
        };
        let abort_signal = Box::pin(async move {
            abort_rx.await.ok();
        });
        server.serve_until(&on_connected, on_process_error, abort_signal).await
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect a client
    let mut client = tokio_modbus::client::tcp::connect(server_addr).await.unwrap();
    
    // Verify client connection works
    let response = client.read_input_registers(0, 1).await.unwrap();
    assert!(response.is_ok());

    // Send abort signal to server
    abort_tx.send(()).unwrap();
    
    // Wait for server to shut down
    let result = timeout(Duration::from_secs(2), server_task).await.unwrap().unwrap();
    println!("Server shutdown result: {:?}", result);
    assert!(matches!(result, Ok(tokio_modbus::server::Terminated::Aborted)));
    
    // Give a moment for connections to close
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Try to use the client again - this should fail because the connection should be closed
    let client_result = timeout(Duration::from_millis(100), client.read_input_registers(0, 1)).await;
    
    // The client should either timeout or get a connection error
    println!("Client result after server shutdown: {:?}", client_result);
    assert!(client_result.is_err() || client_result.unwrap().is_err());
}
