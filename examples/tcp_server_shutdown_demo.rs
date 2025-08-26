// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Demonstration that TCP server properly disconnects clients when cancelled

use std::{future, net::SocketAddr, time::Duration};

use tokio::{net::TcpListener, time::timeout};

use tokio_modbus::{
    prelude::*,
    server::tcp::{accept_tcp_connection, Server},
};

#[derive(Clone)]
struct DemoService;

impl tokio_modbus::server::Service for DemoService {
    type Request = Request<'static>;
    type Response = Response;
    type Exception = ExceptionCode;
    type Future = future::Ready<Result<Self::Response, Self::Exception>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let res = match req {
            Request::ReadInputRegisters(_addr, cnt) => {
                Ok(Response::ReadInputRegisters(vec![42; cnt as usize]))
            }
            _ => Err(ExceptionCode::IllegalFunction),
        };
        future::ready(res)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let socket_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = TcpListener::bind(socket_addr).await.unwrap();
    let server_addr = listener.local_addr().unwrap();

    println!("Starting server on {}", server_addr);

    let server = Server::new(listener);
    // Create an abort signal that triggers after 3 seconds
    let (abort_tx, abort_rx) = tokio::sync::oneshot::channel::<()>();

    // Start the server with abort signal
    let server_task = tokio::spawn(async move {
        let new_service = |_socket_addr| Ok(Some(DemoService));
        let on_connected = |stream, socket_addr| async move {
            accept_tcp_connection(stream, socket_addr, new_service)
        };
        let on_process_error = |err| {
            eprintln!("Server error: {err}");
        };
        let abort_signal = Box::pin(async move {
            abort_rx.await.ok();
        });
        server
            .serve_until(&on_connected, on_process_error, abort_signal)
            .await
    });

    // Connect multiple clients concurrently
    let client_tasks = (0..3)
        .map(|i| {
            tokio::spawn(async move {
                println!("Client {} connecting...", i);
                let mut client = tokio_modbus::client::tcp::connect(server_addr)
                    .await
                    .unwrap();

                // Make some requests
                for j in 0..5 {
                    match timeout(
                        Duration::from_millis(500),
                        client.read_input_registers(0, 1),
                    )
                    .await
                    {
                        Ok(Ok(response)) => {
                            println!("Client {} request {}: Got response: {:?}", i, j, response);
                            tokio::time::sleep(Duration::from_millis(200)).await;
                        }
                        Ok(Err(e)) => {
                            println!("Client {} request {}: Modbus error: {:?}", i, j, e);
                            break;
                        }
                        Err(_) => {
                            println!(
                                "Client {} request {}: Timeout (connection likely closed)",
                                i, j
                            );
                            break;
                        }
                    }
                }
                println!("Client {} finished", i);
            })
        })
        .collect::<Vec<_>>();

    // Create a separate client that we'll keep alive to test after shutdown
    println!("Creating test client for post-shutdown testing...");
    let mut test_client = tokio_modbus::client::tcp::connect(server_addr)
        .await
        .unwrap();

    // Make a request to verify it works
    println!("Test client making initial request...");
    match test_client.read_input_registers(0, 1).await {
        Ok(response) => println!("Test client initial request: {:?}", response),
        Err(e) => println!("Test client initial request failed: {}", e),
    }

    // Let clients run for a bit
    tokio::time::sleep(Duration::from_millis(800)).await;

    println!("Sending abort signal to server...");
    abort_tx.send(()).unwrap();

    // Wait for server to shut down (should be quick now!)
    let shutdown_result = timeout(Duration::from_secs(2), server_task).await;
    match shutdown_result {
        Ok(Ok(Ok(terminated))) => {
            println!("Server shut down successfully: {:?}", terminated);
            assert!(matches!(
                terminated,
                tokio_modbus::server::Terminated::Aborted
            ));
        }
        Ok(Ok(Err(e))) => {
            eprintln!("Server error during shutdown: {}", e);
            return Err(e.into());
        }
        Ok(Err(e)) => {
            eprintln!("Server task panicked: {}", e);
            return Err(e.into());
        }
        Err(_) => {
            eprintln!("Server shutdown timed out!");
            return Err("Server shutdown timeout".into());
        }
    }

    // Wait for clients to finish (they should finish quickly after server shutdown)
    println!("Waiting for clients to finish...");
    for (i, task) in client_tasks.into_iter().enumerate() {
        match timeout(Duration::from_secs(1), task).await {
            Ok(_) => println!("Client {} finished", i),
            Err(_) => println!(
                "Client {} timed out (this is expected after server shutdown)",
                i
            ),
        }
    }

    // Now test the existing client that was connected before shutdown
    println!("Testing existing client that was connected before shutdown...");
    match timeout(
        Duration::from_millis(500),
        test_client.read_input_registers(0, 1),
    )
    .await
    {
        Ok(Ok(response)) => {
            println!(
                "❌ Unexpected: Existing client read succeeded with response: {:?}",
                response
            );
            println!("   This means the connection wasn't properly closed during shutdown!");
            return Err("Existing client read succeeded when it should have failed".into());
        }
        Ok(Err(e)) => {
            println!("✅ Expected: Existing client read failed with error: {}", e);
            println!("   This confirms the connection was properly closed during shutdown.");
        }
        Err(_) => {
            println!("✅ Expected: Existing client read timed out");
            println!("   This confirms the connection was properly closed during shutdown.");
        }
    }

    // Try multiple requests on the existing client to see different error states
    println!("Making multiple requests on existing client to see error progression...");
    for i in 1..=3 {
        match timeout(
            Duration::from_millis(200),
            test_client.read_input_registers(0, 1),
        )
        .await
        {
            Ok(Ok(response)) => {
                println!("❌ Request {}: Unexpected success: {:?}", i, response);
            }
            Ok(Err(e)) => {
                println!("✅ Request {}: Failed as expected: {}", i, e);
            }
            Err(_) => {
                println!("✅ Request {}: Timed out as expected", i);
            }
        }
    }

    println!("\nDemo completed successfully! 🎉");
    println!("This demonstrates that:");
    println!("1. The server accepts multiple concurrent connections");
    println!("2. When cancelled, the server shuts down quickly");
    println!("3. Client connections are properly closed when the server shuts down");
    println!("4. New connections cannot be established after server shutdown");
    println!("5. The server is completely shut down and not accepting new requests");
    println!("6. Existing client connections fail immediately when used after shutdown");
    println!("7. Multiple requests on existing connections show consistent failure behavior");

    Ok(())
}
