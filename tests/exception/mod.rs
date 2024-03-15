// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::future;

use tokio_modbus::{
    client::{Context, Reader, Writer},
    server::Service,
    Exception, Request, Response,
};

pub struct TestService {}

impl TestService {
    fn handle(&self, req: Request<'static>) -> Result<Response, Exception> {
        use Request::*;

        match req {
            ReadCoils(_, _) => Err(Exception::Acknowledge),
            ReadDiscreteInputs(_, _) => Err(Exception::GatewayPathUnavailable),
            WriteSingleCoil(_, _) => Err(Exception::GatewayTargetDevice),
            WriteMultipleCoils(_, _) => Err(Exception::IllegalDataAddress),
            ReadInputRegisters(_, _) => Err(Exception::IllegalDataValue),
            ReadHoldingRegisters(_, _) => Err(Exception::IllegalFunction),
            WriteSingleRegister(_, _) => Err(Exception::MemoryParityError),
            WriteMultipleRegisters(_, _) => Err(Exception::ServerDeviceBusy),
            MaskWriteRegister(_, _, _) => Err(Exception::ServerDeviceFailure),
            _ => Err(Exception::IllegalFunction),
        }
    }
}

impl Service for TestService {
    type Request = Request<'static>;

    type Future = future::Ready<Result<Response, Exception>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        future::ready(self.handle(req))
    }
}

// TODO: Update the `assert_eq` with a check on Exception once Client trait can return Exception
pub async fn check_client_context(mut ctx: Context) {
    let response = ctx.read_coils(0x00, 2).await.expect("communication failed");
    assert_eq!(response, Err(Exception::Acknowledge));

    let response = ctx
        .read_discrete_inputs(0x00, 2)
        .await
        .expect("communication failed");
    assert_eq!(response, Err(Exception::GatewayPathUnavailable));

    let response = ctx
        .write_single_coil(0x00, true)
        .await
        .expect("communication failed");
    assert_eq!(response, Err(Exception::GatewayTargetDevice));

    let response = ctx
        .write_multiple_coils(0x00, &[true])
        .await
        .expect("communication failed");
    assert_eq!(response, Err(Exception::IllegalDataAddress));

    let response = ctx
        .read_input_registers(0x00, 2)
        .await
        .expect("communication failed");
    assert_eq!(response, Err(Exception::IllegalDataValue));

    let response = ctx
        .read_holding_registers(0x00, 2)
        .await
        .expect("communication failed");
    assert_eq!(response, Err(Exception::IllegalFunction));

    let response = ctx
        .write_single_register(0x00, 42)
        .await
        .expect("communication failed");
    assert_eq!(response, Err(Exception::MemoryParityError));

    let response = ctx
        .write_multiple_registers(0x00, &[42])
        .await
        .expect("communication failed");
    assert_eq!(response, Err(Exception::ServerDeviceBusy));

    let response = ctx
        .masked_write_register(0x00, 0, 0)
        .await
        .expect("communication failed");
    assert_eq!(response, Err(Exception::ServerDeviceFailure));

    let response = ctx
        .read_write_multiple_registers(0x00, 0, 0, &[42])
        .await
        .expect("communication failed");
    assert_eq!(response, Err(Exception::IllegalFunction));

    // TODO: This codes hangs if used with `rtu-over-tcp-server`, need to check why
    /*
    let response = ctx.call(Request::Custom(70, Cow::Owned(vec![42]))).await.expect("communication failed");
    assert_eq!(response, Err(Exception::IllegalFunction));
    */
}
