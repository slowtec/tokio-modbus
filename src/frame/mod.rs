use std::{error, fmt};

/// A Modbus function code is represented by an unsigned 8 bit integer.
pub(crate) type FunctionCode = u8;

/// A Modbus address is represented by 16 bit (from `0` to `65535`).
pub(crate) type Address = u16;

/// A Coil represents a single bit.
///
/// - `true` is equivalent to `ON`, `1` and `0xFF00`.
/// - `false` is equivalent to `OFF`, `0` and `0x0000`.
pub(crate) type Coil = bool;

/// Modbus uses 16 bit for its data items (big-endian representation).
pub(crate) type Word = u16;

/// Number of items to process (`0` - `65535`).
pub(crate) type Quantity = u16;

/// A request represents a message from the client (master) to the server (slave).
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    ReadCoils(Vec<Coil>),
    ReadDiscreteInputs(Vec<Coil>),
    WriteSingleCoil(Address),
    WriteMultipleCoils(Address, Quantity),
    ReadInputRegisters(Vec<Word>),
    ReadHoldingRegisters(Vec<Word>),
    WriteSingleRegister(Address, Word),
    WriteMultipleRegisters(Address, Quantity),
    ReadWriteMultipleRegisters(Vec<Word>),
    Custom(FunctionCode, Vec<u8>),
}

/// A server (slave) exception.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exception {
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
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExceptionResponse {
    pub(crate) function: FunctionCode,
    pub(crate) exception: Exception,
}

/// Represents a message from the server (slave) to the client (master).
pub(crate) type ModbusResult = Result<Response, ExceptionResponse>;

/// A modbus [PDU](https://en.wikipedia.org/wiki/Protocol_data_unit)
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Pdu {
    Request(Request),
    Result(ModbusResult),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TcpHeader {
    pub transaction_id: u16,
    pub unit_id: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TcpAdu {
    pub header: TcpHeader,
    pub pdu: Pdu,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RtuAdu {
    pub address: u8,
    pub pdu: Pdu,
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

impl fmt::Display for ExceptionResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Modbus function {}: {}", self.function, self.exception)
    }
}

impl error::Error for ExceptionResponse {
    fn description(&self) -> &str {
        self.exception.description()
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn pdu_from_request() {
        let request = Request::ReadCoils(0x0, 5);
        if let Pdu::Request(Request::ReadCoils(addr, cnt)) = Pdu::from(request) {
            assert_eq!(addr, 0);
            assert_eq!(cnt, 5);
        } else {
            panic!("unexpected result");
        }
    }

    #[test]
    fn pdu_from_response() {
        let response = Response::ReadCoils(vec![true, false]);
        if let Pdu::Result(Ok(Response::ReadCoils(res))) = Pdu::from(response) {
            assert_eq!(res, vec![true, false]);
        } else {
            panic!("unexpected result");
        }
    }

    #[test]
    fn pdu_from_exception_response() {
        let response = ExceptionResponse {
            function: 0x03,
            exception: Exception::IllegalDataValue,
        };
        let mb_result = Err(response);
        if let Pdu::Result(Err(ExceptionResponse {
            function,
            exception,
        })) = Pdu::from(mb_result)
        {
            assert_eq!(function, 0x03);
            assert_eq!(exception, Exception::IllegalDataValue);
        } else {
            panic!("unexpected result");
        }
    }

    #[test]
    fn pdu_from_modbus_result() {
        let response = Response::ReadCoils(vec![true, false]);
        let mb_result = Ok(response);
        if let Pdu::Result(Ok(Response::ReadCoils(res))) = Pdu::from(mb_result) {
            assert_eq!(res, vec![true, false]);
        } else {
            panic!("unexpected result");
        }
    }
}
