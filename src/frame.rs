use std::{error, fmt};

/// A Modbus function code is represented by an unsigned 8 bit integer.
pub type FunctionCode = u8;

/// A Modbus address is represented by 16 bit (from `0` to `65535`).
pub type Address = u16;

/// A Coil represents a single bit.
///
/// - `true` is equivalent to `ON`, `1` and `0xFF00`.
/// - `false` is equivalent to `OFF`, `0` and `0x0000`.
pub type Coil = bool;

/// Modbus uses 16 bit for its data items (big-endian representation).
pub type Word = u16;

/// Number of items to process (`0` - `65535`).
pub type Quantity = u16;

/// A request represents a message from the client (master) to the server (slave).
pub enum Request {
    ReadCoils(Address, Quantity),
    ReadDiscreteInputs(Address, Quantity),
    WriteSingleCoil(Address, Coil),
    WriteMultipleCoils(Address, Vec<Coil>),
    ReadInputRegisters(Address, Quantity),
    ReadHoldingRegisters(Address, Quantity),
    WriteSingleRegister(Address, Word),
    WriteMultipleRegisters(Address, Vec<Word>),
    ReadWriteMultipleRegisters(Address, Quantity, Address, Vec<Word>),
    Custom(FunctionCode, Vec<u8>),
}

/// The data of a successfull request.
pub enum Response {
    ReadCoils(Vec<Coil>),
    ReadDiscreteInputs(Vec<Coil>),
    WriteSingleCoil,
    WriteMultipleCoils,
    ReadInputRegisters(Vec<Word>),
    ReadHoldingRegisters(Vec<Word>),
    WriteSingleRegister,
    WriteMultipleRegisters,
    ReadWriteMultipleRegisters(Vec<Word>),
    Custom(FunctionCode, Vec<u8>),
}

/// A server (slave) exception.
#[derive(Debug)]
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

/// A server (slave) exception response.
pub struct ExceptionResponse(FunctionCode, Exception);

/// Represents a message from the server (slave) to the client (master).
pub type ModbusResult = Result<Response, ExceptionResponse>;

/// A modbus [PDU](https://en.wikipedia.org/wiki/Protocol_data_unit)
pub enum Pdu {
    Request(Request),
    Result(ModbusResult),
}

impl fmt::Display for Exception {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Exception::*;

        match *self {
            IllegalFunction => write!(f, "Illegal function"),
            IllegalDataAddress => write!(f, "Illegal data address"),
            IllegalDataValue => write!(f, "Illegal data value"),
            ServerDeviceFailure => write!(f, "Server device failure"),
            Acknowledge => write!(f, "Acknowledge"),
            ServerDeviceBusy => write!(f, "Server device busy"),
            MemoryParityError => write!(f, "Memory parity error"),
            GatewayPathUnavailable => write!(f, "Gateway path unavailable"),
            GatewayTargetDevice => write!(f, "Gateway target device failed to respond"),
        }
    }
}

impl error::Error for Exception {
    fn description(&self) -> &str {
        use self::Exception::*;

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

impl From<Request> for Pdu {
    fn from(req: Request) -> Pdu {
        Pdu::Request(req)
    }
}

impl From<Response> for Pdu {
    fn from(res: Response) -> Pdu {
        Pdu::Result(Ok(res))
    }
}

impl From<ExceptionResponse> for Pdu {
    fn from(ex: ExceptionResponse) -> Pdu {
        Pdu::Result(Err(ex))
    }
}

impl From<ModbusResult> for Pdu {
    fn from(res: ModbusResult) -> Pdu {
        Pdu::Result(res)
    }
}
