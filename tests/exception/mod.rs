// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::future;

use tokio_modbus::{
    client::{Context, Reader as _, Writer as _},
    server::Service,
    ExceptionCode, Request, Response,
};

pub struct TestService {}

impl TestService {
    fn handle(&self, req: Request<'static>) -> Result<Response, ExceptionCode> {
        use Request::*;

        match req {
            ReadCoils(_, _) => Err(ExceptionCode::Acknowledge),
            ReadDiscreteInputs(_, _) => Err(ExceptionCode::GatewayPathUnavailable),
            WriteSingleCoil(_, _) => Err(ExceptionCode::GatewayTargetDevice),
            WriteMultipleCoils(_, _) => Err(ExceptionCode::IllegalDataAddress),
            ReadInputRegisters(_, _) => Err(ExceptionCode::IllegalDataValue),
            ReadHoldingRegisters(_, _) => Err(ExceptionCode::IllegalFunction),
            WriteSingleRegister(_, _) => Err(ExceptionCode::MemoryParityError),
            WriteMultipleRegisters(_, _) => Err(ExceptionCode::ServerDeviceBusy),
            MaskWriteRegister(_, _, _) => Err(ExceptionCode::ServerDeviceFailure),
            _ => Err(ExceptionCode::IllegalFunction),
        }
    }
}

impl Service for TestService {
    type Request = Request<'static>;

    type Response = Response;

    type Exception = ExceptionCode;

    type Future = future::Ready<Result<Self::Response, Self::Exception>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        future::ready(self.handle(req))
    }
}

// TODO: Update the `assert_eq` with a check on Exception once Client trait can return Exception
pub async fn check_client_context(mut ctx: Context) {
    let response = ctx.read_coils(0x00, 2).await.unwrap();
    assert!(matches!(response, Err(ExceptionCode::Acknowledge)));

    let response = ctx
        .read_discrete_inputs(0x00, 2)
        .await
        .expect("communication failed");
    assert!(matches!(
        response,
        Err(ExceptionCode::GatewayPathUnavailable)
    ));

    let response = ctx
        .write_single_coil(0x00, true)
        .await
        .expect("communication failed");
    assert!(matches!(response, Err(ExceptionCode::GatewayTargetDevice)));

    let response = ctx
        .write_multiple_coils(0x00, &[true])
        .await
        .expect("communication failed");
    assert!(matches!(response, Err(ExceptionCode::IllegalDataAddress)));

    let response = ctx
        .read_input_registers(0x00, 2)
        .await
        .expect("communication failed");
    assert!(matches!(response, Err(ExceptionCode::IllegalDataValue)));

    let response = ctx
        .read_holding_registers(0x00, 2)
        .await
        .expect("communication failed");
    assert!(matches!(response, Err(ExceptionCode::IllegalFunction)));

    let response = ctx
        .write_single_register(0x00, 42)
        .await
        .expect("communication failed");
    assert!(matches!(response, Err(ExceptionCode::MemoryParityError)));

    let response = ctx
        .write_multiple_registers(0x00, &[42])
        .await
        .expect("communication failed");
    assert!(matches!(response, Err(ExceptionCode::ServerDeviceBusy)));

    let response = ctx
        .masked_write_register(0x00, 0, 0)
        .await
        .expect("communication failed");
    assert!(matches!(response, Err(ExceptionCode::ServerDeviceFailure)));

    let response = ctx
        .read_write_multiple_registers(0x00, 0, 0, &[42])
        .await
        .expect("communication failed");
    assert!(matches!(response, Err(ExceptionCode::IllegalFunction)));

    // TODO: This codes hangs if used with `rtu-over-tcp-server`, need to check why
    // let response = ctx
    //     .call(Request::Custom(70, Cow::Owned(vec![42])))
    //     .await
    //     .expect("communication failed");
    // assert!(matches!(
    //     response,
    //     Err(Exception::IllegalFunction)
    // ));
}
