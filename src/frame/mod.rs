// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(feature = "rtu")]
pub(crate) mod rtu;

#[cfg(feature = "tcp")]
pub(crate) mod tcp;

use std::{
    borrow::Cow,
    error,
    fmt::{self, Display},
};

use crate::bytes::Bytes;

/// MEI type code (`0x0E`) for Modbus "Read Device Identification" (function 0x2B).
pub(crate) const MEI_TYPE_READ_DEVICE_IDENTIFICATION: u8 = 0x0E;

/// A Modbus function code.
///
/// All function codes as defined by the protocol specification V1.1b3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionCode {
    /// 01 (0x01) Read Coils.
    ReadCoils,

    /// 02 (0x02) Read Discrete Inputs
    ReadDiscreteInputs,

    /// 03 (0x03) Read Holding Registers
    ReadHoldingRegisters,

    /// 04 (0x04) Read Input Registers
    ReadInputRegisters,

    /// 05 (0x05) Write Single Coil
    WriteSingleCoil,

    /// 06 (0x06) Write Single Register
    WriteSingleRegister,

    /// 07 (0x07) Read Exception Status (Serial Line only)
    ReadExceptionStatus,

    /// 08 (0x08) Diagnostics (Serial Line only)
    Diagnostics,

    /// 11 (0x0B) Get Comm Event Counter (Serial Line only)
    GetCommEventCounter,

    /// 12 (0x0C) Get Comm Event Log (Serial Line only)
    GetCommEventLog,

    /// 15 (0x0F) Write Multiple Coils
    WriteMultipleCoils,

    /// 16 (0x10) Write Multiple Registers
    WriteMultipleRegisters,

    /// 17 (0x11) Report Slave ID (Serial Line only)
    ReportServerId,

    /// 20 (0x14) Read File Record
    ReadFileRecord,

    /// 21 (0x15) Write File Record
    WriteFileRecord,

    /// 22 (0x16) Mask Write Register
    MaskWriteRegister,

    /// 23 (0x17) Read/Write Multiple Registers
    ReadWriteMultipleRegisters,

    /// 24 (0x18) Read FIFO Queue
    ReadFifoQueue,

    /// 43 ( 0x2B) Encapsulated Interface Transport
    EncapsulatedInterfaceTransport,

    /// Custom Modbus Function Code.
    Custom(u8),
}

impl FunctionCode {
    /// Create a new [`FunctionCode`] with `value`.
    #[must_use]
    pub const fn new(value: u8) -> Self {
        match value {
            0x01 => Self::ReadCoils,
            0x02 => Self::ReadDiscreteInputs,
            0x03 => Self::ReadHoldingRegisters,
            0x04 => Self::ReadInputRegisters,
            0x05 => Self::WriteSingleCoil,
            0x06 => Self::WriteSingleRegister,
            0x07 => Self::ReadExceptionStatus,
            0x08 => Self::Diagnostics,
            0x0B => Self::GetCommEventCounter,
            0x0C => Self::GetCommEventLog,
            0x0F => Self::WriteMultipleCoils,
            0x10 => Self::WriteMultipleRegisters,
            0x11 => Self::ReportServerId,
            0x14 => Self::ReadFileRecord,
            0x15 => Self::WriteFileRecord,
            0x16 => Self::MaskWriteRegister,
            0x17 => Self::ReadWriteMultipleRegisters,
            0x18 => Self::ReadFifoQueue,
            0x2B => Self::EncapsulatedInterfaceTransport,
            code => Self::Custom(code),
        }
    }

    /// Gets the [`u8`] value of the current [`FunctionCode`].
    #[must_use]
    pub const fn value(self) -> u8 {
        match self {
            Self::ReadCoils => 0x01,
            Self::ReadDiscreteInputs => 0x02,
            Self::ReadHoldingRegisters => 0x03,
            Self::ReadInputRegisters => 0x04,
            Self::WriteSingleCoil => 0x05,
            Self::WriteSingleRegister => 0x06,
            Self::ReadExceptionStatus => 0x07,
            Self::Diagnostics => 0x08,
            Self::GetCommEventCounter => 0x0B,
            Self::GetCommEventLog => 0x0C,
            Self::WriteMultipleCoils => 0x0F,
            Self::WriteMultipleRegisters => 0x10,
            Self::ReportServerId => 0x11,
            Self::ReadFileRecord => 0x14,
            Self::WriteFileRecord => 0x15,
            Self::MaskWriteRegister => 0x16,
            Self::ReadWriteMultipleRegisters => 0x17,
            Self::ReadFifoQueue => 0x18,
            Self::EncapsulatedInterfaceTransport => 0x2B,
            Self::Custom(code) => code,
        }
    }
}

impl Display for FunctionCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value().fmt(f)
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

    /// A request to report server ID (Serial Line only).
    ReportServerId,

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

    /// A request to read device identification.
    /// The first parameter is the [`ReadCode`].
    /// The second parameter is the object ID: the first object to return (stream access)
    /// or the specific object to read (individual access).
    ReadDeviceIdentification(ReadCode, ObjectId),

    /// A raw Modbus request.
    /// The first parameter is the Modbus function code.
    /// The second parameter is the raw bytes of the request.
    Custom(u8, Cow<'a, [u8]>),
}

impl Request<'_> {
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
            ReportServerId => ReportServerId,
            MaskWriteRegister(addr, and_mask, or_mask) => {
                MaskWriteRegister(addr, and_mask, or_mask)
            }
            ReadWriteMultipleRegisters(addr, qty, write_addr, words) => {
                ReadWriteMultipleRegisters(addr, qty, write_addr, Cow::Owned(words.into_owned()))
            }
            ReadDeviceIdentification(read_code, object_id) => {
                ReadDeviceIdentification(read_code, object_id)
            }

            Custom(func, bytes) => Custom(func, Cow::Owned(bytes.into_owned())),
        }
    }

    /// Get the [`FunctionCode`] of the [`Request`].
    #[must_use]
    pub const fn function_code(&self) -> FunctionCode {
        use Request::*;

        match self {
            ReadCoils(_, _) => FunctionCode::ReadCoils,
            ReadDiscreteInputs(_, _) => FunctionCode::ReadDiscreteInputs,

            WriteSingleCoil(_, _) => FunctionCode::WriteSingleCoil,
            WriteMultipleCoils(_, _) => FunctionCode::WriteMultipleCoils,

            ReadInputRegisters(_, _) => FunctionCode::ReadInputRegisters,
            ReadHoldingRegisters(_, _) => FunctionCode::ReadHoldingRegisters,

            WriteSingleRegister(_, _) => FunctionCode::WriteSingleRegister,
            WriteMultipleRegisters(_, _) => FunctionCode::WriteMultipleRegisters,

            ReportServerId => FunctionCode::ReportServerId,

            MaskWriteRegister(_, _, _) => FunctionCode::MaskWriteRegister,

            ReadWriteMultipleRegisters(_, _, _, _) => FunctionCode::ReadWriteMultipleRegisters,

            ReadDeviceIdentification(_, _) => FunctionCode::EncapsulatedInterfaceTransport,

            Custom(code, _) => FunctionCode::Custom(*code),
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
impl SlaveRequest<'_> {
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
    /// Response to a `ReadCoils` request
    /// The parameter contains the coil values that have been read
    /// See also the note above regarding the vector length
    ReadCoils(Vec<Coil>),

    /// Response to a `ReadDiscreteInputs` request
    /// The parameter contains the discrete input values that have been read
    /// See also the note above regarding the vector length
    ReadDiscreteInputs(Vec<Coil>),

    /// Response to a `WriteSingleCoil` request
    /// The first parameter contains the address of the coil that has been written to
    /// The second parameter contains the value that has been written to the coil the given address
    WriteSingleCoil(Address, Coil),

    /// Response to a `WriteMultipleCoils` request
    /// The first parameter contains the address at the start of the range that has been written to
    /// The second parameter contains the amount of values that have been written
    WriteMultipleCoils(Address, Quantity),

    /// Response to a `ReadInputRegisters` request
    /// The parameter contains the register values that have been read
    ReadInputRegisters(Vec<Word>),

    /// Response to a `ReadHoldingRegisters` request
    /// The parameter contains the register values that have been read
    ReadHoldingRegisters(Vec<Word>),

    /// Response to a `WriteSingleRegister` request
    /// The first parameter contains the address of the register that has been written to
    /// The second parameter contains the value that has been written to the register at the given address
    WriteSingleRegister(Address, Word),

    /// Response to a `WriteMultipleRegisters` request
    /// The first parameter contains the address at the start of the register range that has been written to
    /// The second parameter contains the amount of register that have been written
    WriteMultipleRegisters(Address, Quantity),

    /// Response to a `ReportServerId` request
    /// The first parameter contains the server ID
    /// The second parameter indicates whether the server is running
    /// The third parameter contains additional data from the server
    ReportServerId(u8, bool, Vec<u8>),

    /// Response `MaskWriteRegister`
    /// The first parameter is the address of the holding register.
    /// The second parameter is the AND mask.
    /// The third parameter is the OR mask.
    MaskWriteRegister(Address, Word, Word),

    /// Response to a `ReadWriteMultipleRegisters` request
    /// The parameter contains the register values that have been read as part of the read instruction
    ReadWriteMultipleRegisters(Vec<Word>),

    /// Response to a `ReadDeviceIdentification` request
    /// The first parameter is the [`ReadCode`] used in the request
    /// The second parameter is the device's [`ConformityLevel`]
    /// The third parameter indicates whether more objects follow in a subsequent response ([`MoreFollows`])
    /// The fourth parameter is the ID of the next object ([`NextObjectId`]) to request, if any
    /// The fifth parameter contains the list of identification objects returned ([`DeviceIdObjects`])
    ReadDeviceIdentification(
        ReadCode,
        ConformityLevel,
        MoreFollows,
        NextObjectId,
        DeviceIdObjects,
    ),

    /// Response to a raw Modbus request
    /// The first parameter contains the returned Modbus function code
    /// The second parameter contains the bytes read following the function code
    Custom(u8, Bytes),
}

impl Response {
    /// Get the [`FunctionCode`] of the [`Response`].
    #[must_use]
    pub const fn function_code(&self) -> FunctionCode {
        use Response::*;

        match self {
            ReadCoils(_) => FunctionCode::ReadCoils,
            ReadDiscreteInputs(_) => FunctionCode::ReadDiscreteInputs,

            WriteSingleCoil(_, _) => FunctionCode::WriteSingleCoil,
            WriteMultipleCoils(_, _) => FunctionCode::WriteMultipleCoils,

            ReadInputRegisters(_) => FunctionCode::ReadInputRegisters,
            ReadHoldingRegisters(_) => FunctionCode::ReadHoldingRegisters,

            WriteSingleRegister(_, _) => FunctionCode::WriteSingleRegister,
            WriteMultipleRegisters(_, _) => FunctionCode::WriteMultipleRegisters,

            ReportServerId(_, _, _) => FunctionCode::ReportServerId,

            MaskWriteRegister(_, _, _) => FunctionCode::MaskWriteRegister,

            ReadWriteMultipleRegisters(_) => FunctionCode::ReadWriteMultipleRegisters,

            ReadDeviceIdentification(_, _, _, _, _) => FunctionCode::EncapsulatedInterfaceTransport,

            Custom(code, _) => FunctionCode::Custom(*code),
        }
    }
}

/// A server (slave) exception.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionCode {
    /// 0x01
    IllegalFunction,
    /// 0x02
    IllegalDataAddress,
    /// 0x03
    IllegalDataValue,
    /// 0x04
    ServerDeviceFailure,
    /// 0x05
    Acknowledge,
    /// 0x06
    ServerDeviceBusy,
    /// 0x08
    MemoryParityError,
    /// 0x0A
    GatewayPathUnavailable,
    /// 0x0B
    GatewayTargetDevice,
    /// None of the above.
    ///
    /// Although encoding one of the predefined values as this is possible, it is not recommended.
    /// Instead, prefer to use [`Self::new()`] to prevent such ambiguities.
    Custom(u8),
}

impl From<ExceptionCode> for u8 {
    fn from(from: ExceptionCode) -> Self {
        use crate::frame::ExceptionCode::*;
        match from {
            IllegalFunction => 0x01,
            IllegalDataAddress => 0x02,
            IllegalDataValue => 0x03,
            ServerDeviceFailure => 0x04,
            Acknowledge => 0x05,
            ServerDeviceBusy => 0x06,
            MemoryParityError => 0x08,
            GatewayPathUnavailable => 0x0A,
            GatewayTargetDevice => 0x0B,
            Custom(code) => code,
        }
    }
}

impl ExceptionCode {
    /// Create a new [`ExceptionCode`] with `value`.
    #[must_use]
    pub const fn new(value: u8) -> Self {
        use crate::frame::ExceptionCode::*;

        match value {
            0x01 => IllegalFunction,
            0x02 => IllegalDataAddress,
            0x03 => IllegalDataValue,
            0x04 => ServerDeviceFailure,
            0x05 => Acknowledge,
            0x06 => ServerDeviceBusy,
            0x08 => MemoryParityError,
            0x0A => GatewayPathUnavailable,
            0x0B => GatewayTargetDevice,
            other => Custom(other),
        }
    }

    pub(crate) fn description(&self) -> &str {
        use crate::frame::ExceptionCode::*;

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
            Custom(_) => "Custom",
        }
    }
}

/// Represents the Modbus read device identification access type.
///
/// Used to specify the type of information to retrieve from a device during a
/// "Read Device Identification" Modbus function (0x2B / 0x0E).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadCode {
    /// Basic identification (stream access).
    /// Corresponds to value `0x01`. Returns a minimal set of identification data.
    Basic,
    /// Regular identification (stream access).
    /// Corresponds to value `0x02`. Returns additional identification beyond basic.
    Regular,
    /// Extended identification (stream access).
    /// Corresponds to value `0x03`. Returns the most comprehensive set of device info.
    Extended,
    /// Specific identification (individual access).
    /// Corresponds to value `0x04`. Used to retrieve a specific object by ID.
    Specific,
}

impl ReadCode {
    /// Attempts to convert a raw [`u8`] value to a [`ReadCode`].
    ///
    /// # Parameters
    /// - `value`: The raw byte representing the read code.
    ///
    /// # Returns
    /// - `Some(ReadCode)` if the value is valid.
    /// - `None` otherwise.
    pub const fn try_from_value(value: u8) -> Option<Self> {
        Some(match value {
            0x01 => ReadCode::Basic,
            0x02 => ReadCode::Regular,
            0x03 => ReadCode::Extended,
            0x04 => ReadCode::Specific,
            _ => return None,
        })
    }

    /// Returns the [`u8`] representation of the current [`ReadCode`] variant.
    ///
    /// # Returns
    /// A byte corresponding to the Modbus function read code.
    pub const fn value(self) -> u8 {
        match self {
            ReadCode::Basic => 0x01,
            ReadCode::Regular => 0x02,
            ReadCode::Extended => 0x03,
            ReadCode::Specific => 0x04,
        }
    }
}
/// Represents the conformity level of a Modbus device's identification support.
///
/// Indicates what types of identification objects a device supports,
/// and whether access is limited to stream access or includes individual access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformityLevel {
    /// Only basic identification objects via stream access (`0x01`).
    BasicIdentificationStreamOnly,

    /// Only regular identification objects via stream access (`0x02`).
    RegularIdentificationStreamOnly,

    /// Only extended identification objects via stream access (`0x03`).
    ExtendedIdentificationStreamOnly,

    /// Basic identification objects, with individual access supported (`0x81`).
    BasicIdentification,

    /// Regular identification objects, with individual access supported (`0x82`).
    RegularIdentification,

    /// Extended identification objects, with individual access supported (`0x83`).
    ExtendedIdentification,
}

impl ConformityLevel {
    /// Attempts to convert a raw [`u8`] to a [`ConformityLevel`].
    ///
    /// # Parameters
    /// - `value`: The raw byte representing the device's conformity level.
    ///
    /// # Returns
    /// - `Some(ConformityLevel)` if the value matches a known level.
    /// - `None` for unrecognized values.
    pub const fn try_from_value(value: u8) -> Option<Self> {
        Some(match value {
            0x01 => ConformityLevel::BasicIdentificationStreamOnly,
            0x02 => ConformityLevel::RegularIdentificationStreamOnly,
            0x03 => ConformityLevel::ExtendedIdentificationStreamOnly,
            0x81 => ConformityLevel::BasicIdentification,
            0x82 => ConformityLevel::RegularIdentification,
            0x83 => ConformityLevel::ExtendedIdentification,
            _ => return None,
        })
    }

    /// Returns the [`u8`] representation of the current [`ConformityLevel`] variant.
    ///
    /// # Returns
    /// A byte that can be used in Modbus device identification responses.
    pub const fn value(self) -> u8 {
        match self {
            ConformityLevel::BasicIdentificationStreamOnly => 0x01,
            ConformityLevel::RegularIdentificationStreamOnly => 0x02,
            ConformityLevel::ExtendedIdentificationStreamOnly => 0x03,
            ConformityLevel::BasicIdentification => 0x81,
            ConformityLevel::RegularIdentification => 0x82,
            ConformityLevel::ExtendedIdentification => 0x83,
        }
    }
}

/// Identifier of a single device ID object.
///
/// Each object represents a specific type of information (e.g., vendor name, product code).
pub type ObjectId = u8;

/// Indicates whether more identification objects follow in the response.
///
/// `true` means more data is available; `false` means this is the final part.
pub type MoreFollows = bool;

/// Specifies the ID of the next object to be requested in case of partial data.
///
/// Used when `MoreFollows` is `true`, should be 0 otherwise.
pub type NextObjectId = u8;

/// A vector of device identification objects ([`DeviceIdObject`]).
///
/// Each [`DeviceIdObject`] in the list represents a specific piece of identification
/// information such as the vendor name, product code, or firmware version.
///
/// The object values are returned as raw bytes and can optionally be interpreted
/// as UTF-8 strings using [`DeviceIdObject::value_as_str`].
///
/// This list is typically received in response to a Modbus "Read Device Identification"
/// ([`Request::ReadDeviceIdentification`]) request.
pub type DeviceIdObjects = Vec<DeviceIdObject>;

/// Represents a single Modbus device identification object.
///
/// Each object consists of an ID and an associated binary value,
/// typically representing device metadata such as vendor name,
/// product code, or software version.
///
/// The value is stored as raw bytes and may be interpreted as a UTF-8 string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdObject {
    /// Object identifier (0x00 to 0xFF), as defined by the Modbus specification.
    ///
    /// Common IDs include:
    /// - 0x00: VendorName
    /// - 0x01: ProductCode
    /// - 0x02: MajorMinorRevision
    pub id: u8,

    /// Raw byte value associated with this object.
    ///
    /// May contain UTF-8-encoded strings or other binary data.
    pub value: Bytes,
}

impl DeviceIdObject {
    /// Attempts to interpret the object's value as a UTF-8 string.
    ///
    /// This is useful when the identification object contains human-readable
    /// data, such as a vendor name or software version.
    ///
    /// # Returns
    /// - `Some(&str)` if the value is valid UTF-8.
    /// - `None` if the value contains invalid UTF-8 bytes.
    pub fn value_as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.value).ok()
    }
}

/// A server (slave) exception response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExceptionResponse {
    pub function: FunctionCode,
    pub exception: ExceptionCode,
}

/// Represents a message from the client (slave) to the server (master).
#[derive(Debug, Clone)]
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
pub(crate) struct ResponsePdu(pub(crate) Result<Response, ExceptionResponse>);

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

#[cfg(any(
    feature = "rtu-over-tcp-server",
    feature = "rtu-server",
    feature = "tcp-server"
))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptionalResponsePdu(pub(crate) Option<ResponsePdu>);

#[cfg(any(
    feature = "rtu-over-tcp-server",
    feature = "rtu-server",
    feature = "tcp-server"
))]
impl From<Result<Option<Response>, ExceptionResponse>> for OptionalResponsePdu {
    fn from(from: Result<Option<Response>, ExceptionResponse>) -> Self {
        match from {
            Ok(None) => Self(None),
            Ok(Some(response)) => Self(Some(response.into())),
            Err(exception) => Self(Some(exception.into())),
        }
    }
}

impl From<ResponsePdu> for Result<Response, ExceptionResponse> {
    fn from(from: ResponsePdu) -> Self {
        from.0
    }
}

impl fmt::Display for ExceptionCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl error::Error for ExceptionCode {
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
    fn new_function_code() {
        assert_eq!(FunctionCode::ReadCoils, FunctionCode::new(0x01));
        assert_eq!(FunctionCode::ReadDiscreteInputs, FunctionCode::new(0x02));

        assert_eq!(FunctionCode::WriteSingleCoil, FunctionCode::new(0x05));
        assert_eq!(FunctionCode::WriteSingleRegister, FunctionCode::new(0x06));

        assert_eq!(FunctionCode::ReadHoldingRegisters, FunctionCode::new(0x03));
        assert_eq!(FunctionCode::ReadInputRegisters, FunctionCode::new(0x04));

        assert_eq!(FunctionCode::WriteMultipleCoils, FunctionCode::new(0x0F));
        assert_eq!(
            FunctionCode::WriteMultipleRegisters,
            FunctionCode::new(0x10)
        );

        assert_eq!(FunctionCode::MaskWriteRegister, FunctionCode::new(0x016));

        assert_eq!(
            FunctionCode::ReadWriteMultipleRegisters,
            FunctionCode::new(0x017)
        );

        assert_eq!(FunctionCode::Custom(70), FunctionCode::new(70));
    }

    #[test]
    fn function_code_values() {
        assert_eq!(FunctionCode::ReadCoils.value(), 0x01);
        assert_eq!(FunctionCode::ReadDiscreteInputs.value(), 0x02);

        assert_eq!(FunctionCode::WriteSingleCoil.value(), 0x05);
        assert_eq!(FunctionCode::WriteSingleRegister.value(), 0x06);

        assert_eq!(FunctionCode::ReadHoldingRegisters.value(), 0x03);
        assert_eq!(FunctionCode::ReadInputRegisters.value(), 0x04);

        assert_eq!(FunctionCode::WriteMultipleCoils.value(), 0x0F);
        assert_eq!(FunctionCode::WriteMultipleRegisters.value(), 0x10);

        assert_eq!(FunctionCode::MaskWriteRegister.value(), 0x016);

        assert_eq!(FunctionCode::ReadWriteMultipleRegisters.value(), 0x017);

        assert_eq!(FunctionCode::Custom(70).value(), 70);
    }

    #[test]
    fn function_code_from_request() {
        use Request::*;

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

        assert_eq!(Custom(88, Cow::Borrowed(&[])).function_code().value(), 88);
    }

    #[test]
    fn function_code_from_response() {
        use Response::*;

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
            Custom(99, Bytes::from_static(&[])).function_code().value(),
            99
        );
    }
}
