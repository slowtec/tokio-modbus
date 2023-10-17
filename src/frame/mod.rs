// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(feature = "rtu")]
pub(crate) mod rtu;

#[cfg(feature = "tcp")]
pub(crate) mod tcp;

use std::{borrow::Cow, error, fmt};

use crate::bytes::Bytes;

/// A Modbus function code is represented by an unsigned 8 bit integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionCode {
    ReadCoils,                  //=> 0x01,
    ReadDiscreteInputs,         // => 0x02,
    WriteSingleCoil,            //(_, _) => 0x05,
    WriteMultipleCoils,         //(_, _) => 0x0F,
    ReadInputRegisters,         //(_) => 0x04,
    ReadHoldingRegisters,       //(_) => 0x03,
    WriteSingleRegister,        //(_, _) => 0x06,
    WriteMultipleRegisters,     //(_, _) => 0x10,
    MaskWriteRegister,          //(_, _, _) => 0x16,
    ReadWriteMultipleRegisters, //(_) => 0x17,
    Custom(u8),                 // => code,
}

impl From<FunctionCode> for u8 {
    fn from(val: FunctionCode) -> u8 {
        match val {
            FunctionCode::ReadCoils => 0x01,
            FunctionCode::ReadDiscreteInputs => 0x02,
            FunctionCode::WriteSingleCoil => 0x05,
            FunctionCode::WriteMultipleCoils => 0x0f,
            FunctionCode::ReadInputRegisters => 0x04,
            FunctionCode::ReadHoldingRegisters => 0x03,
            FunctionCode::WriteSingleRegister => 0x06,
            FunctionCode::WriteMultipleRegisters => 0x10,
            FunctionCode::MaskWriteRegister => 0x16,
            FunctionCode::ReadWriteMultipleRegisters => 0x17,
            FunctionCode::Custom(code) => code,
        }
    }
}

impl From<u8> for FunctionCode {
    fn from(value: u8) -> Self {
        match value {
            0x01 => FunctionCode::ReadCoils,
            0x02 => FunctionCode::ReadDiscreteInputs,
            0x05 => FunctionCode::WriteSingleCoil,
            0x0f => FunctionCode::WriteMultipleCoils,
            0x04 => FunctionCode::ReadInputRegisters,
            0x03 => FunctionCode::ReadHoldingRegisters,
            0x06 => FunctionCode::WriteSingleRegister,
            0x10 => FunctionCode::WriteMultipleRegisters,
            0x16 => FunctionCode::MaskWriteRegister,
            0x17 => FunctionCode::ReadWriteMultipleRegisters,
            value => FunctionCode::Custom(value),
        }
    }
}

impl std::fmt::Display for FunctionCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionCode::ReadCoils => f.write_str("FunctionCode::ReadCoils"),
            FunctionCode::ReadDiscreteInputs => f.write_str("FunctionCode::ReadDiscreteInputs"),
            FunctionCode::WriteSingleCoil => f.write_str("FunctionCode::WriteSingleCoil"),
            FunctionCode::WriteMultipleCoils => f.write_str("FunctionCode::WriteMultipleCoils"),
            FunctionCode::ReadInputRegisters => f.write_str("FunctionCode::ReadInputRegisters"),
            FunctionCode::ReadHoldingRegisters => f.write_str("FunctionCode::ReadHoldingRegisters"),
            FunctionCode::WriteSingleRegister => f.write_str("FunctionCode::WriteSingleRegister"),
            FunctionCode::WriteMultipleRegisters => {
                f.write_str("FunctionCode::WriteMultipleRegisters")
            }
            FunctionCode::MaskWriteRegister => f.write_str("FunctionCode::MaskWriteRegister"),
            FunctionCode::ReadWriteMultipleRegisters => {
                f.write_str("FunctionCode::ReadWriteMultipleRegisters")
            }
            FunctionCode::Custom(value) => {
                f.write_str(format!("FunctionCode::Custom({value})").as_str())
            }
        }
    }
}

/// A Modbus protocol address is represented by 16 bit from `0` to `65535`.
///
/// This *protocol address* uses 0-based indexing, while the *coil address* or
/// *register address* is often specified as a number with 1-based indexing.
/// Please consult the specification of your devices if 1-based coil/register
/// addresses need to be converted to 0-based protocol addresses by subtracting 1.
pub type Address = u16;

/// A Coil represents a single bit.
///
/// - `true` is equivalent to `ON`, `1` and `0xFF00`.
/// - `false` is equivalent to `OFF`, `0` and `0x0000`.
pub(crate) type Coil = bool;

/// Modbus uses 16 bit for its data items.
///
/// Transmitted using a big-endian representation.
pub(crate) type Word = u16;

/// Number of items to process.
pub type Quantity = u16;

/// A Trait that is implemented on types that have a function code
pub trait GetFunctionCode {
    fn function_code(&self) -> FunctionCode;
}

/// A request represents a message from the client (master) to the server (slave).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request<'a> {
    /// A request to read multiple coils.
    /// The first parameter is the address of the first coil to read.
    /// The second parameter is the number of coils to read.
    ReadCoils(Address, Quantity),

    /// A request to read multiple discrete inputs
    /// The first parameter is the address of the first discrete input to read.
    /// The second parameter is the number of discrete inputs to read.
    ReadDiscreteInputs(Address, Quantity),

    /// A request to write a single coil.
    /// The first parameter is the address of the coil.
    /// The second parameter is the value to write to the coil.
    WriteSingleCoil(Address, Coil),

    /// A request to write multiple coils.
    /// The first parameter is the address of the first coil to write.
    /// The second parameter is the vector of values to write to the coils.
    WriteMultipleCoils(Address, Cow<'a, [Coil]>),

    /// A request to read multiple input registers.
    /// The first parameter is the address of the first input register to read.
    /// The second parameter is the number of input registers to read.
    ReadInputRegisters(Address, Quantity),

    /// A request to read multiple holding registers.
    /// The first parameter is the address of the first holding register to read.
    /// The second parameter is the number of holding registers to read.
    ReadHoldingRegisters(Address, Quantity),

    /// A request to write a single register.
    /// The first parameter is the address of the register to read.
    /// The second parameter is the value to write to the register.
    WriteSingleRegister(Address, Word),

    /// A request to write to multiple registers.
    /// The first parameter is the address of the first register to write.
    /// The second parameter is the vector of values to write to the registers.
    WriteMultipleRegisters(Address, Cow<'a, [Word]>),

    /// A request to set or clear individual bits of a holding register.
    /// The first parameter is the address of the holding register.
    /// The second parameter is the AND mask.
    /// The third parameter is the OR mask.
    MaskWriteRegister(Address, Word, Word),

    /// A request to simultaneously read multiple registers and write multiple registers.
    /// The first parameter is the address of the first register to read.
    /// The second parameter is the number of registers to read.
    /// The third parameter is the address of the first register to write.
    /// The fourth parameter is the vector of values to write to the registers.
    ReadWriteMultipleRegisters(Address, Quantity, Address, Cow<'a, [Word]>),

    /// A raw Modbus request.
    /// The first parameter is the Modbus function code.
    /// The second parameter is the raw bytes of the request.
    Custom(FunctionCode, Cow<'a, [u8]>),

    /// A poison pill for stopping the client service and to release
    /// the underlying transport, e.g. for disconnecting from an
    /// exclusively used serial port.
    ///
    /// This is an ugly workaround, because `tokio-proto` does not
    /// provide other means to gracefully shut down the `ClientService`.
    /// Otherwise the bound transport is never freed as long as the
    /// executor is active, even when dropping the Modbus client
    /// context.
    Disconnect,
}

impl<'a> Request<'a> {
    /// Converts the request into an owned instance with `'static'` lifetime.
    #[must_use]
    pub fn into_owned(self) -> Request<'static> {
        use Request::*;

        match self {
            ReadCoils(addr, qty) => ReadCoils(addr, qty),
            ReadDiscreteInputs(addr, qty) => ReadDiscreteInputs(addr, qty),
            WriteSingleCoil(addr, coil) => WriteSingleCoil(addr, coil),
            WriteMultipleCoils(addr, coils) => {
                WriteMultipleCoils(addr, Cow::Owned(coils.into_owned()))
            }
            ReadInputRegisters(addr, qty) => ReadInputRegisters(addr, qty),
            ReadHoldingRegisters(addr, qty) => ReadHoldingRegisters(addr, qty),
            WriteSingleRegister(addr, word) => WriteSingleRegister(addr, word),
            WriteMultipleRegisters(addr, words) => {
                WriteMultipleRegisters(addr, Cow::Owned(words.into_owned()))
            }
            MaskWriteRegister(addr, and_mask, or_mask) => {
                MaskWriteRegister(addr, and_mask, or_mask)
            }
            ReadWriteMultipleRegisters(addr, qty, write_addr, words) => {
                ReadWriteMultipleRegisters(addr, qty, write_addr, Cow::Owned(words.into_owned()))
            }
            Custom(func, bytes) => Custom(func, Cow::Owned(bytes.into_owned())),
            Disconnect => Disconnect,
        }
    }

    // pub fn into_exception(self, exception: Exception) -> ExceptionResponse{

    // }
}

impl<'a> GetFunctionCode for Request<'a> {
    fn function_code(&self) -> FunctionCode {
        use Request::*;
        match *self {
            ReadCoils(_, _) => FunctionCode::ReadCoils,
            ReadDiscreteInputs(_, _) => FunctionCode::ReadDiscreteInputs,
            WriteSingleCoil(_, _) => FunctionCode::WriteSingleCoil,
            WriteMultipleCoils(_, _) => FunctionCode::WriteMultipleCoils,
            ReadInputRegisters(_, _) => FunctionCode::ReadInputRegisters,
            ReadHoldingRegisters(_, _) => FunctionCode::ReadHoldingRegisters,
            WriteSingleRegister(_, _) => FunctionCode::WriteSingleRegister,
            WriteMultipleRegisters(_, _) => FunctionCode::WriteMultipleRegisters,
            MaskWriteRegister(_, _, _) => FunctionCode::MaskWriteRegister,
            ReadWriteMultipleRegisters(_, _, _, _) => FunctionCode::ReadWriteMultipleRegisters,
            Custom(code, _) => code,
            Disconnect => unreachable!(),
        }
    }
}

/// A Modbus request with slave included
#[cfg(feature = "server")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlaveRequest<'a> {
    /// Slave Id from the request
    pub slave: crate::slave::SlaveId,
    /// A `Request` enum
    pub request: Request<'a>,
}

#[cfg(feature = "server")]
impl<'a> SlaveRequest<'a> {
    /// Converts the request into an owned instance with `'static'` lifetime.
    #[must_use]
    pub fn into_owned(self) -> SlaveRequest<'static> {
        let Self { slave, request } = self;
        SlaveRequest {
            slave,
            request: request.into_owned(),
        }
    }
}

/// The data of a successful request.
///
/// ReadCoils/ReadDiscreteInputs: The length of the result Vec is always a
/// multiple of 8. Only the values of the first bits/coils that have actually
/// been requested are defined. The value of the remaining bits depend on the
/// server implementation and those coils should be should be ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// Response to a ReadCoils request
    /// The parameter contains the coil values that have been read
    /// See also the note above regarding the vector length
    ReadCoils(Vec<Coil>),

    /// Response to a ReadDiscreteInputs request
    /// The parameter contains the discrete input values that have been read
    /// See also the note above regarding the vector length
    ReadDiscreteInputs(Vec<Coil>),

    /// Response to a WriteSingleCoil request
    /// The first parameter contains the address of the coil that has been written to
    /// The second parameter contains the value that has been written to the coil the given address
    WriteSingleCoil(Address, Coil),

    /// Response to a WriteMultipleCoils request
    /// The first parameter contains the address at the start of the range that has been written to
    /// The second parameter contains the amount of values that have been written
    WriteMultipleCoils(Address, Quantity),

    /// Response to a ReadInputRegisters request
    /// The parameter contains the register values that have been read
    ReadInputRegisters(Vec<Word>),

    /// Response to a ReadHoldingRegisters request
    /// The parameter contains the register values that have been read
    ReadHoldingRegisters(Vec<Word>),

    /// Response to a WriteSingleRegister request
    /// The first parameter contains the address of the register that has been written to
    /// The second parameter contains the value that has been written to the register at the given address
    WriteSingleRegister(Address, Word),

    /// Response to a WriteMultipleRegisters request
    /// The first parameter contains the address at the start of the register range that has been written to
    /// The second parameter contains the amount of register that have been written
    WriteMultipleRegisters(Address, Quantity),

    /// Response MaskWriteRegister
    /// The first parameter is the address of the holding register.
    /// The second parameter is the AND mask.
    /// The third parameter is the OR mask.
    MaskWriteRegister(Address, Word, Word),

    /// Response to a ReadWriteMultipleRegisters request
    /// The parameter contains the register values that have been read as part of the read instruction
    ReadWriteMultipleRegisters(Vec<Word>),

    /// Response to a raw Modbus request
    /// The first parameter contains the returned Modbus function code
    /// The second parameter contains the bytes read following the function code
    Custom(FunctionCode, Bytes),
}

impl GetFunctionCode for Response {
    fn function_code(&self) -> FunctionCode {
        use Response::*;
        match *self {
            ReadCoils(_) => FunctionCode::ReadCoils,
            ReadDiscreteInputs(_) => FunctionCode::ReadDiscreteInputs,
            WriteSingleCoil(_, _) => FunctionCode::WriteSingleCoil,
            WriteMultipleCoils(_, _) => FunctionCode::WriteMultipleCoils,
            ReadInputRegisters(_) => FunctionCode::ReadInputRegisters,
            ReadHoldingRegisters(_) => FunctionCode::ReadHoldingRegisters,
            WriteSingleRegister(_, _) => FunctionCode::WriteSingleRegister,
            WriteMultipleRegisters(_, _) => FunctionCode::WriteMultipleRegisters,
            MaskWriteRegister(_, _, _) => FunctionCode::MaskWriteRegister,
            ReadWriteMultipleRegisters(_) => FunctionCode::ReadWriteMultipleRegisters,
            Custom(code, _) => code,
        }
    }
}

/// A server (slave) exception.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Exception {
    IllegalFunction = 0x01,
    IllegalDataAddress = 0x02,
    IllegalDataValue = 0x03,
    ServerDeviceFailure = 0x04,
    Acknowledge = 0x05,
    ServerDeviceBusy = 0x06,
    MemoryParityError = 0x08,
    GatewayPathUnavailable = 0x0A,
    GatewayTargetDevice = 0x0B,
}

impl From<Exception> for u8 {
    fn from(from: Exception) -> Self {
        from as u8
    }
}

impl Exception {
    pub(crate) fn description(&self) -> &str {
        use crate::frame::Exception::*;

        match *self {
            IllegalFunction => "Illegal function",
            IllegalDataAddress => "Illegal data address",
            IllegalDataValue => "Illegal data value",
            ServerDeviceFailure => "Server device failure",
            Acknowledge => "Acknowledge",
            ServerDeviceBusy => "Server device busy",
            MemoryParityError => "Memory parity error",
            GatewayPathUnavailable => "Gateway path unavailable",
            GatewayTargetDevice => "Gateway target device failed to respond",
        }
    }
}

/// A server (slave) exception response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExceptionResponse {
    pub function: FunctionCode,
    pub exception: Exception,
}

/// Convenience trait for downcasting `std::io::Error` to `ExceptionResponse`
pub trait ExtractExceptionResponse {
    fn exception_response(self) -> Result<ExceptionResponse, std::io::Error>;
}

fn try_downcast_error<T>(error: std::io::Error) -> Result<T, std::io::Error>
where
    T: error::Error + Send + Sync + 'static,
{
    match error
        .get_ref()
        .map(<(dyn std::error::Error + std::marker::Send + std::marker::Sync + 'static)>::is::<T>)
    {
        Some(true) => Ok(*error.into_inner().unwrap().downcast::<T>().unwrap()),
        _ => Err(error),
    }
}

impl ExtractExceptionResponse for std::io::Error {
    fn exception_response(self) -> Result<ExceptionResponse, std::io::Error> {
        try_downcast_error(self)
    }
}

/// Represents a message from the client (slave) to the server (master).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestPdu<'a>(pub(crate) Request<'a>);

impl<'a> From<Request<'a>> for RequestPdu<'a> {
    fn from(from: Request<'a>) -> Self {
        RequestPdu(from)
    }
}

impl<'a> From<RequestPdu<'a>> for Request<'a> {
    fn from(from: RequestPdu<'a>) -> Self {
        from.0
    }
}

/// Represents a message from the server (slave) to the client (master).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponsePdu(pub(crate) Result<Response, ExceptionResponse>);

impl From<Response> for ResponsePdu {
    fn from(from: Response) -> Self {
        ResponsePdu(Ok(from))
    }
}

impl From<ExceptionResponse> for ResponsePdu {
    fn from(from: ExceptionResponse) -> Self {
        ResponsePdu(Err(from))
    }
}

impl From<Result<Response, ExceptionResponse>> for ResponsePdu {
    fn from(from: Result<Response, ExceptionResponse>) -> Self {
        ResponsePdu(from.map(Into::into).map_err(Into::into))
    }
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalResponsePdu(pub(crate) Option<ResponsePdu>);

#[cfg(feature = "server")]
impl<T> From<Option<T>> for OptionalResponsePdu
where
    T: Into<ResponsePdu>,
{
    fn from(from: Option<T>) -> Self {
        Self(from.map(Into::into))
    }
}

#[cfg(feature = "server")]
impl<T> From<T> for OptionalResponsePdu
where
    T: Into<ResponsePdu>,
{
    fn from(from: T) -> Self {
        Self(Some(from.into()))
    }
}

impl From<ResponsePdu> for Result<Response, ExceptionResponse> {
    fn from(from: ResponsePdu) -> Self {
        from.0
    }
}

impl fmt::Display for Exception {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl error::Error for Exception {
    fn description(&self) -> &str {
        self.description()
    }
}

impl fmt::Display for ExceptionResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Modbus function {}: {}", self.function, self.exception)
    }
}

impl error::Error for ExceptionResponse {
    fn description(&self) -> &str {
        self.exception.description()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_code_from_request() {
        use crate::frame::Request::*;
        assert_eq!(ReadCoils(0, 0).function_code(), FunctionCode::ReadCoils);
        assert_eq!(
            ReadDiscreteInputs(0, 0).function_code(),
            FunctionCode::ReadDiscreteInputs
        );
        assert_eq!(
            WriteSingleCoil(0, true).function_code(),
            FunctionCode::WriteSingleCoil
        );
        assert_eq!(
            WriteMultipleCoils(0, Cow::Borrowed(&[])).function_code(),
            FunctionCode::WriteMultipleCoils
        );
        assert_eq!(
            ReadInputRegisters(0, 0).function_code(),
            FunctionCode::ReadInputRegisters
        );
        assert_eq!(
            ReadHoldingRegisters(0, 0).function_code(),
            FunctionCode::ReadHoldingRegisters
        );
        assert_eq!(
            WriteSingleRegister(0, 0).function_code(),
            FunctionCode::WriteSingleRegister
        );
        assert_eq!(
            WriteMultipleRegisters(0, Cow::Borrowed(&[])).function_code(),
            FunctionCode::WriteMultipleRegisters
        );
        assert_eq!(
            MaskWriteRegister(0, 0, 0).function_code(),
            FunctionCode::MaskWriteRegister
        );
        assert_eq!(
            ReadWriteMultipleRegisters(0, 0, 0, Cow::Borrowed(&[])).function_code(),
            FunctionCode::ReadWriteMultipleRegisters
        );
        assert_eq!(
            Custom(FunctionCode::Custom(88), Cow::Borrowed(&[])).function_code(),
            FunctionCode::Custom(88)
        );
    }

    #[test]
    fn function_code_from_response() {
        use crate::frame::Response::*;
        assert_eq!(ReadCoils(vec![]).function_code(), FunctionCode::ReadCoils);
        assert_eq!(
            ReadDiscreteInputs(vec![]).function_code(),
            FunctionCode::ReadDiscreteInputs
        );
        assert_eq!(
            WriteSingleCoil(0x0, false).function_code(),
            FunctionCode::WriteSingleCoil
        );
        assert_eq!(
            WriteMultipleCoils(0x0, 0x0).function_code(),
            FunctionCode::WriteMultipleCoils
        );
        assert_eq!(
            ReadInputRegisters(vec![]).function_code(),
            FunctionCode::ReadInputRegisters
        );
        assert_eq!(
            ReadHoldingRegisters(vec![]).function_code(),
            FunctionCode::ReadHoldingRegisters
        );
        assert_eq!(
            WriteSingleRegister(0, 0).function_code(),
            FunctionCode::WriteSingleRegister
        );
        assert_eq!(
            WriteMultipleRegisters(0, 0).function_code(),
            FunctionCode::WriteMultipleRegisters
        );
        assert_eq!(
            MaskWriteRegister(0, 0, 0).function_code(),
            FunctionCode::MaskWriteRegister
        );
        assert_eq!(
            ReadWriteMultipleRegisters(vec![]).function_code(),
            FunctionCode::ReadWriteMultipleRegisters
        );
        assert_eq!(
            Custom(FunctionCode::Custom(99), Bytes::from_static(&[])).function_code(),
            FunctionCode::Custom(99)
        );
    }
}
