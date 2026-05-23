// SPDX-FileCopyrightText: Copyright (c) 2017-2026 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus RTU server skeleton

use std::{future::Future, io, path::Path};

use futures_util::{FutureExt as _, SinkExt as _, StreamExt as _};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_serial::SerialStream;
use tokio_util::codec::Framed;

use crate::{
    codec::rtu::ServerCodec,
    frame::{
        ExceptionResponse, OptionalResponsePdu, RequestPdu,
        rtu::{RequestAdu, ResponseAdu},
    },
    slave::SlaveId,
};

use super::{Service, Terminated};

#[derive(Debug)]
pub struct Server {
    serial: SerialStream,
    slave_id: Option<SlaveId>,
}

impl Server {
    /// set up a new [`Server`] instance from an interface path and baud rate
    pub fn new_from_path<P: AsRef<Path>>(p: P, baud_rate: u32) -> io::Result<Self> {
        let serial =
            SerialStream::open(&tokio_serial::new(p.as_ref().to_string_lossy(), baud_rate))?;
        Ok(Server {
            serial,
            slave_id: None,
        })
    }

    /// set up a new [`Server`] instance based on a pre-configured [`SerialStream`] instance
    #[must_use]
    pub fn new(serial: SerialStream) -> Self {
        Server {
            serial,
            slave_id: None,
        }
    }

    /// Configure the server to only respond to requests addressed to the given slave ID.
    ///
    /// Requests for other slave IDs are silently ignored at the transport layer,
    /// as required by the Modbus RTU specification for devices on a shared RS-485 bus.
    ///
    /// By default (without calling this method), the server responds to all requests
    /// regardless of slave ID.
    #[must_use]
    pub fn with_slave_id(mut self, slave_id: impl Into<SlaveId>) -> Self {
        self.slave_id = Some(slave_id.into());
        self
    }

    /// Process Modbus RTU requests.
    pub async fn serve_forever<S>(self, service: S) -> io::Result<()>
    where
        S: Service + Send + Sync + 'static,
        S::Request: From<RequestAdu<'static>> + Send,
    {
        let framed = Framed::new(self.serial, ServerCodec::default());
        process(framed, service, self.slave_id).await
    }

    /// Process Modbus RTU requests until finished or aborted.
    ///
    /// Warning: Request processing is not scoped and could be aborted at any internal await point!
    /// See also: <https://rust-lang.github.io/wg-async/vision/roadmap/scopes.html#cancellation>
    pub async fn serve_until<S, X>(self, service: S, abort_signal: X) -> io::Result<Terminated>
    where
        S: Service + Send + Sync + 'static,
        S::Request: From<RequestAdu<'static>> + Send,
        X: Future<Output = ()> + Sync + Send + 'static,
    {
        let framed = Framed::new(self.serial, ServerCodec::default());
        let abort_signal = abort_signal.fuse();
        tokio::select! {
            res = process(framed, service, self.slave_id) => {
                res.map(|()| Terminated::Finished)
            },
            () = abort_signal => {
                Ok(Terminated::Aborted)
            }
        }
    }
}

/// frame wrapper around the underlying service's responses to forwarded requests
async fn process<S, T>(
    mut framed: Framed<T, ServerCodec>,
    service: S,
    slave_id: Option<SlaveId>,
) -> io::Result<()>
where
    S: Service + Send + Sync + 'static,
    S::Request: From<RequestAdu<'static>> + Send,
    T: AsyncRead + AsyncWrite + Unpin,
{
    loop {
        let Some(request_adu) = framed.next().await.transpose().inspect_err(|err| {
            log::debug!("Failed to receive and decode request ADU: {err}");
        })?
        else {
            log::debug!("Stream has finished");
            break;
        };

        let RequestAdu {
            hdr,
            pdu: RequestPdu(request),
        } = &request_adu;
        let hdr = *hdr;

        if let Some(id) = slave_id {
            if hdr.slave_id != id {
                log::trace!(
                    "Ignoring request for slave {}, expected {}",
                    hdr.slave_id,
                    id
                );
                continue;
            }
        }

        let fc = request.function_code();
        let OptionalResponsePdu(Some(response_pdu)) = service
            .call(request_adu.into())
            .await
            .map(Into::into)
            .map_err(|e| ExceptionResponse {
                function: fc,
                exception: e.into(),
            })
            .into()
        else {
            log::trace!("No response for request {hdr:?} (function = {fc})");
            continue;
        };

        framed
            .send(ResponseAdu {
                hdr,
                pdu: response_pdu,
            })
            .await
            .inspect_err(|err| {
                log::debug!("Failed to send response for request {hdr:?} (function = {fc}): {err}");
            })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{future, time::Duration};

    use tokio::net::{TcpListener, TcpStream};

    use crate::{
        client,
        client::Reader as _,
        prelude::{ExceptionCode, Request, Response, Slave},
        server::Service,
    };

    struct TestService;

    impl Service for TestService {
        type Request = Request<'static>;
        type Response = Response;
        type Exception = ExceptionCode;
        type Future = future::Ready<Result<Self::Response, Self::Exception>>;

        fn call(&self, req: Self::Request) -> Self::Future {
            match req {
                Request::ReadInputRegisters(_addr, cnt) => {
                    let mut registers = vec![0; cnt.into()];
                    registers[0] = 0x42;
                    future::ready(Ok(Response::ReadInputRegisters(registers)))
                }
                _ => future::ready(Err(ExceptionCode::IllegalFunction)),
            }
        }
    }

    async fn run_server(listener: TcpListener, slave_id: Option<SlaveId>) -> io::Result<()> {
        let (stream, _addr) = listener.accept().await?;
        let framed = Framed::new(stream, ServerCodec::default());
        process(framed, TestService, slave_id).await
    }

    #[tokio::test]
    async fn responds_to_matching_slave_id() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let slave = Slave(1);

        tokio::select! {
            _ = run_server(listener, Some(slave.into())) => unreachable!(),
            () = async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                let transport = TcpStream::connect(addr).await.unwrap();
                let mut ctx = client::rtu::attach_slave(transport, slave);
                let rsp = ctx.read_input_registers(0x00, 3).await.unwrap();
                assert_eq!(rsp.unwrap(), vec![0x42, 0x0, 0x0]);
            } => (),
        }
    }

    #[tokio::test]
    async fn ignores_mismatched_slave_id() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::select! {
            // Server configured for slave 1
            _ = run_server(listener, Some(Slave(1).into())) => unreachable!(),
            () = async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                // Client sends request to slave 2 — server should NOT respond
                let transport = TcpStream::connect(addr).await.unwrap();
                let mut ctx = client::rtu::attach_slave(transport, Slave(2));
                let result = tokio::time::timeout(
                    Duration::from_millis(500),
                    ctx.read_input_registers(0x00, 3),
                ).await;
                // Should timeout because the server silently ignores the request
                assert!(result.is_err(), "Expected timeout, but got a response");
            } => (),
        }
    }

    #[tokio::test]
    async fn without_slave_id_responds_to_all() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::select! {
            // Server without slave ID filter — should respond to any slave
            _ = run_server(listener, None) => unreachable!(),
            () = async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                let transport = TcpStream::connect(addr).await.unwrap();
                let mut ctx = client::rtu::attach_slave(transport, Slave(42));
                let rsp = ctx.read_input_registers(0x00, 3).await.unwrap();
                assert_eq!(rsp.unwrap(), vec![0x42, 0x0, 0x0]);
            } => (),
        }
    }
}
