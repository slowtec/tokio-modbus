// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{borrow::Cow, future};

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
    let response = ctx.read_coils(0x00, 2).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().to_string(),
        format!(
            "Modbus function {}: {}",
            Request::ReadCoils(0, 0).function_code(),
            Exception::Acknowledge
        ),
    );

    let response = ctx.read_discrete_inputs(0x00, 2).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().to_string(),
        format!(
            "Modbus function {}: {}",
            Request::ReadDiscreteInputs(0, 0).function_code(),
            Exception::GatewayPathUnavailable
        ),
    );

    let response = ctx.write_single_coil(0x00, true).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().to_string(),
        format!(
            "Modbus function {}: {}",
            Request::WriteSingleCoil(0, true).function_code(),
            Exception::GatewayTargetDevice
        ),
    );

    let response = ctx.write_multiple_coils(0x00, &[true]).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().to_string(),
        format!(
            "Modbus function {}: {}",
            Request::WriteMultipleCoils(0, Cow::Owned(vec![true])).function_code(),
            Exception::IllegalDataAddress
        ),
    );

    let response = ctx.read_input_registers(0x00, 2).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().to_string(),
        format!(
            "Modbus function {}: {}",
            Request::ReadInputRegisters(0, 2).function_code(),
            Exception::IllegalDataValue
        ),
    );

    let response = ctx.read_holding_registers(0x00, 2).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().to_string(),
        format!(
            "Modbus function {}: {}",
            Request::ReadHoldingRegisters(0, 2).function_code(),
            Exception::IllegalFunction
        ),
    );

    let response = ctx.write_single_register(0x00, 42).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().to_string(),
        format!(
            "Modbus function {}: {}",
            Request::WriteSingleRegister(0, 42).function_code(),
            Exception::MemoryParityError
        ),
    );

    let response = ctx.write_multiple_registers(0x00, &[42]).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().to_string(),
        format!(
            "Modbus function {}: {}",
            Request::WriteMultipleRegisters(0, Cow::Owned(vec![42])).function_code(),
            Exception::ServerDeviceBusy
        ),
    );

    let response = ctx.masked_write_register(0x00, 0, 0).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().to_string(),
        format!(
            "Modbus function {}: {}",
            Request::MaskWriteRegister(0, 0, 0).function_code(),
            Exception::ServerDeviceFailure
        ),
    );

    let response = ctx.read_write_multiple_registers(0x00, 0, 0, &[42]).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().to_string(),
        format!(
            "Modbus function {}: {}",
            Request::ReadWriteMultipleRegisters(0, 0, 0, Cow::Owned(vec![42])).function_code(),
            Exception::IllegalFunction
        ),
    );

    // TODO: This codes hangs if used with `rtu-over-tcp-server`, need to check why
    /*let response = ctx.call(Request::Custom(70, Cow::Owned(vec![42]))).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().to_string(),
        format!(
            "Modbus function {}: {}",
            Request::Custom(70, Cow::Owned(vec![42])).function_code(),
            Exception::IllegalFunction
        ),
    );*/
}
