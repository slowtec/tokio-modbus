#![allow(deprecated)]
#[cfg(feature = "rtu")]
pub mod rtu;

#[cfg(feature = "tcp")]
pub mod tcp;

use crate::frame::*;

use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, Bytes, BytesMut};
use std::convert::TryFrom;
use std::io::{self, Cursor, Error, ErrorKind};

impl From<Request> for Bytes {
    fn from(req: Request) -> Bytes {
        let cnt = request_byte_count(&req);
        let mut data = BytesMut::with_capacity(cnt);
        use crate::frame::Request::*;
        data.put_u8(req_to_fn_code(&req));
        match req {
            ReadCoils(address, quantity)
            | ReadDiscreteInputs(address, quantity)
            | ReadInputRegisters(address, quantity)
            | ReadHoldingRegisters(address, quantity) => {
                data.put_u16(address);
                data.put_u16(quantity);
            }
            WriteSingleCoil(address, state) => {
                data.put_u16(address);
                data.put_u16(bool_to_coil(state));
            }
            WriteMultipleCoils(address, coils) => {
                data.put_u16(address);
                let len = coils.len();
                data.put_u16(len as u16);
                let packed_coils = pack_coils(&coils);
                data.put_u8(packed_coils.len() as u8);
                for b in packed_coils {
                    data.put_u8(b);
                }
            }
            WriteSingleRegister(address, word) => {
                data.put_u16(address);
                data.put_u16(word);
            }
            WriteMultipleRegisters(address, words) => {
                data.put_u16(address);
                let len = words.len();
                data.put_u16(len as u16);
                data.put_u8((len as u8) * 2);
                for w in words {
                    data.put_u16(w);
                }
            }
            ReadWriteMultipleRegisters(read_address, quantity, write_address, words) => {
                data.put_u16(read_address);
                data.put_u16(quantity);
                data.put_u16(write_address);
                let n = words.len();
                data.put_u16(n as u16);
                data.put_u8(n as u8 * 2);
                for w in words {
                    data.put_u16(w);
                }
            }
            Custom(_, custom_data) => {
                for d in custom_data {
                    data.put_u8(d);
                }
            }
            Disconnect => unreachable!(),
        }
        data.freeze()
    }
}

impl From<RequestPdu> for Bytes {
    fn from(pdu: RequestPdu) -> Bytes {
        pdu.0.into()
    }
}

impl From<Response> for Bytes {
    fn from(rsp: Response) -> Bytes {
        let cnt = response_byte_count(&rsp);
        let mut data = BytesMut::with_capacity(cnt);
        use crate::frame::Response::*;
        data.put_u8(rsp_to_fn_code(&rsp));
        match rsp {
            ReadCoils(coils) | ReadDiscreteInputs(coils) => {
                let packed_coils = pack_coils(&coils);
                data.put_u8(packed_coils.len() as u8);
                for b in packed_coils {
                    data.put_u8(b);
                }
            }
            ReadInputRegisters(registers)
            | ReadHoldingRegisters(registers)
            | ReadWriteMultipleRegisters(registers) => {
                data.put_u8((registers.len() * 2) as u8);
                for r in registers {
                    data.put_u16(r);
                }
            }
            WriteSingleCoil(address, state) => {
                data.put_u16(address);
                data.put_u16(bool_to_coil(state));
            }
            WriteMultipleCoils(address, quantity) | WriteMultipleRegisters(address, quantity) => {
                data.put_u16(address);
                data.put_u16(quantity);
            }
            WriteSingleRegister(address, word) => {
                data.put_u16(address);
                data.put_u16(word);
            }
            Custom(_, custom_data) => {
                for d in custom_data {
                    data.put_u8(d);
                }
            }
        }
        data.freeze()
    }
}

impl From<ExceptionResponse> for Bytes {
    fn from(ex: ExceptionResponse) -> Bytes {
        let mut data = BytesMut::with_capacity(2);
        debug_assert!(ex.function < 0x80);
        data.put_u8(ex.function + 0x80);
        data.put_u8(ex.exception as u8);
        data.freeze()
    }
}

impl From<ResponsePdu> for Bytes {
    fn from(pdu: ResponsePdu) -> Bytes {
        // TODO: Replace with Result::map_or_else() when available
        pdu.0.map(Into::into).unwrap_or_else(Into::into)
    }
}

impl TryFrom<Bytes> for Request {
    type Error = Error;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        use crate::frame::Request::*;
        let mut rdr = Cursor::new(&bytes);
        let fn_code = rdr.read_u8()?;
        let req = match fn_code {
            0x01 => ReadCoils(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?),
            0x02 => ReadDiscreteInputs(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?),
            0x05 => WriteSingleCoil(
                rdr.read_u16::<BigEndian>()?,
                coil_to_bool(rdr.read_u16::<BigEndian>()?)?,
            ),
            0x0F => {
                let address = rdr.read_u16::<BigEndian>()?;
                let quantity = rdr.read_u16::<BigEndian>()?;
                let byte_count = rdr.read_u8()?;
                if bytes.len() < (6 + byte_count as usize) {
                    return Err(Error::new(ErrorKind::InvalidData, "Invalid byte count"));
                }
                let x = &bytes[6..];
                WriteMultipleCoils(address, unpack_coils(x, quantity))
            }
            0x04 => ReadInputRegisters(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?),
            0x03 => {
                ReadHoldingRegisters(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?)
            }
            0x06 => WriteSingleRegister(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?),

            0x10 => {
                let address = rdr.read_u16::<BigEndian>()?;
                let quantity = rdr.read_u16::<BigEndian>()?;
                let byte_count = rdr.read_u8()? as usize;
                if bytes.len() < (6 + byte_count as usize) {
                    return Err(Error::new(ErrorKind::InvalidData, "Invalid byte count"));
                }
                let mut data = vec![];
                for _ in 0..quantity {
                    data.push(rdr.read_u16::<BigEndian>()?);
                }
                WriteMultipleRegisters(address, data)
            }
            0x17 => {
                let read_address = rdr.read_u16::<BigEndian>()?;
                let read_quantity = rdr.read_u16::<BigEndian>()?;
                let write_address = rdr.read_u16::<BigEndian>()?;
                let write_quantity = rdr.read_u16::<BigEndian>()?;
                let write_count = rdr.read_u8()? as usize;
                let mut data = vec![];
                if bytes.len() < (10 + write_count as usize) {
                    return Err(Error::new(ErrorKind::InvalidData, "Invalid byte count"));
                }
                for _ in 0..write_quantity {
                    data.push(rdr.read_u16::<BigEndian>()?);
                }
                ReadWriteMultipleRegisters(read_address, read_quantity, write_address, data)
            }
            fn_code if fn_code < 0x80 => Custom(fn_code, bytes[1..].into()),
            fn_code => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Invalid function code: 0x{:0>2X}", fn_code),
                ));
            }
        };
        Ok(req)
    }
}

impl TryFrom<Bytes> for RequestPdu {
    type Error = Error;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        let pdu = Request::try_from(bytes)?.into();
        Ok(pdu)
    }
}

impl TryFrom<Bytes> for Response {
    type Error = Error;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        use crate::frame::Response::*;
        let mut rdr = Cursor::new(&bytes);
        let fn_code = rdr.read_u8()?;
        let rsp = match fn_code {
            0x01 => {
                let byte_count = rdr.read_u8()?;
                let x = &bytes[2..];
                // Here we have not information about the exact requested quantity so we just
                // unpack the whole byte.
                let quantity = u16::from(byte_count * 8);
                ReadCoils(unpack_coils(x, quantity))
            }
            0x02 => {
                let byte_count = rdr.read_u8()?;
                let x = &bytes[2..];
                // Here we have no information about the exact requested quantity so we just
                // unpack the whole byte.
                let quantity = u16::from(byte_count * 8);
                ReadDiscreteInputs(unpack_coils(x, quantity))
            }
            0x05 => WriteSingleCoil(rdr.read_u16::<BigEndian>()?, coil_to_bool(rdr.read_u16::<BigEndian>()?)?,),
            0x0F => WriteMultipleCoils(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?),
            0x04 => {
                let byte_count = rdr.read_u8()?;
                let quantity = byte_count / 2;
                let mut data = vec![];
                for _ in 0..quantity {
                    data.push(rdr.read_u16::<BigEndian>()?);
                }
                ReadInputRegisters(data)
            }
            0x03 => {
                let byte_count = rdr.read_u8()?;
                let quantity = byte_count / 2;
                let mut data = vec![];
                for _ in 0..quantity {
                    data.push(rdr.read_u16::<BigEndian>()?);
                }
                ReadHoldingRegisters(data)
            }
            0x06 => WriteSingleRegister(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?),

            0x10 => {
                WriteMultipleRegisters(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?)
            }
            0x17 => {
                let byte_count = rdr.read_u8()?;
                let quantity = byte_count / 2;
                let mut data = vec![];
                for _ in 0..quantity {
                    data.push(rdr.read_u16::<BigEndian>()?);
                }
                ReadWriteMultipleRegisters(data)
            }
            _ => Custom(fn_code, bytes[1..].into()),
        };
        Ok(rsp)
    }
}

impl TryFrom<Bytes> for ExceptionResponse {
    type Error = Error;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        let mut rdr = Cursor::new(&bytes);
        let fn_err_code = rdr.read_u8()?;
        if fn_err_code < 0x80 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Invalid exception function code",
            ));
        }
        let function = fn_err_code - 0x80;
        let exception = Exception::try_from(rdr.read_u8()?)?;
        Ok(ExceptionResponse {
            function,
            exception,
        })
    }
}

impl TryFrom<u8> for Exception {
    type Error = Error;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        use crate::frame::Exception::*;
        let ex = match code {
            0x01 => IllegalFunction,
            0x02 => IllegalDataAddress,
            0x03 => IllegalDataValue,
            0x04 => ServerDeviceFailure,
            0x05 => Acknowledge,
            0x06 => ServerDeviceBusy,
            0x08 => MemoryParityError,
            0x0A => GatewayPathUnavailable,
            0x0B => GatewayTargetDevice,
            _ => {
                return Err(Error::new(ErrorKind::InvalidData, "Invalid exception code"));
            }
        };
        Ok(ex)
    }
}

impl TryFrom<Bytes> for ResponsePdu {
    type Error = Error;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        let fn_code = Cursor::new(&bytes).read_u8()?;
        let pdu = if fn_code < 0x80 {
            Response::try_from(bytes)?.into()
        } else {
            ExceptionResponse::try_from(bytes)?.into()
        };
        Ok(pdu)
    }
}

fn bool_to_coil(state: bool) -> u16 {
    if state {
        0xFF00
    } else {
        0x0000
    }
}

fn coil_to_bool(coil: u16) -> io::Result<bool> {
    match coil {
        0xFF00 => Ok(true),
        0x0000 => Ok(false),
        _ => Err(Error::new(ErrorKind::InvalidData, "Invalid coil value: {}")),
    }
}

fn packed_coils_len(bitcount: usize) -> usize {
    (bitcount + 7) / 8
}

fn pack_coils(coils: &[Coil]) -> Vec<u8> {
    let packed_size = packed_coils_len(coils.len());
    let mut res = vec![0; packed_size];
    for (i, b) in coils.iter().enumerate() {
        let v = if *b { 0b1 } else { 0b0 };
        res[(i / 8) as usize] |= v << (i % 8);
    }
    res
}

fn unpack_coils(bytes: &[u8], count: u16) -> Vec<Coil> {
    let mut res = Vec::with_capacity(count as usize);
    for i in 0..count {
        res.push((bytes[(i / 8u16) as usize] >> (i % 8)) & 0b1 > 0);
    }
    res
}

fn req_to_fn_code(req: &Request) -> u8 {
    use crate::frame::Request::*;
    match *req {
        ReadCoils(_, _) => 0x01,
        ReadDiscreteInputs(_, _) => 0x02,
        WriteSingleCoil(_, _) => 0x05,
        WriteMultipleCoils(_, _) => 0x0F,
        ReadInputRegisters(_, _) => 0x04,
        ReadHoldingRegisters(_, _) => 0x03,
        WriteSingleRegister(_, _) => 0x06,
        WriteMultipleRegisters(_, _) => 0x10,
        ReadWriteMultipleRegisters(_, _, _, _) => 0x17,
        Custom(code, _) => code,
        Disconnect => unreachable!(),
    }
}

fn rsp_to_fn_code(rsp: &Response) -> u8 {
    use crate::frame::Response::*;
    match *rsp {
        ReadCoils(_) => 0x01,
        ReadDiscreteInputs(_) => 0x02,
        WriteSingleCoil(_, _) => 0x05,
        WriteMultipleCoils(_, _) => 0x0F,
        ReadInputRegisters(_) => 0x04,
        ReadHoldingRegisters(_) => 0x03,
        WriteSingleRegister(_, _) => 0x06,
        WriteMultipleRegisters(_, _) => 0x10,
        ReadWriteMultipleRegisters(_) => 0x17,
        Custom(code, _) => code,
    }
}

fn request_byte_count(req: &Request) -> usize {
    use crate::frame::Request::*;
    match *req {
        ReadCoils(_, _)
        | ReadDiscreteInputs(_, _)
        | ReadInputRegisters(_, _)
        | ReadHoldingRegisters(_, _)
        | WriteSingleRegister(_, _)
        | WriteSingleCoil(_, _) => 5,
        WriteMultipleCoils(_, ref coils) => 6 + packed_coils_len(coils.len()),
        WriteMultipleRegisters(_, ref data) => 6 + data.len() * 2,
        ReadWriteMultipleRegisters(_, _, _, ref data) => 10 + data.len() * 2,
        Custom(_, ref data) => 1 + data.len(),
        Disconnect => unreachable!(),
    }
}

fn response_byte_count(rsp: &Response) -> usize {
    use crate::frame::Response::*;
    match *rsp {
        ReadCoils(ref coils) | ReadDiscreteInputs(ref coils) => 2 + packed_coils_len(coils.len()),
        WriteSingleCoil(_, _) |
        WriteMultipleCoils(_, _) | WriteMultipleRegisters(_, _) | WriteSingleRegister(_, _) => 5,
        ReadInputRegisters(ref data)
        | ReadHoldingRegisters(ref data)
        | ReadWriteMultipleRegisters(ref data) => 2 + data.len() * 2,
        Custom(_, ref data) => 1 + data.len(),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn convert_bool_to_coil() {
        assert_eq!(bool_to_coil(true), 0xFF00);
        assert_eq!(bool_to_coil(false), 0x0000);
    }

    #[test]
    fn convert_coil_to_bool() {
        assert_eq!(coil_to_bool(0xFF00).unwrap(), true);
        assert_eq!(coil_to_bool(0x0000).unwrap(), false);
    }

    #[test]
    fn convert_booleans_to_bytes() {
        assert_eq!(pack_coils(&[]), &[]);
        assert_eq!(pack_coils(&[true]), &[0b_1]);
        assert_eq!(pack_coils(&[false]), &[0b_0]);
        assert_eq!(pack_coils(&[true, false]), &[0b_01]);
        assert_eq!(pack_coils(&[false, true]), &[0b_10]);
        assert_eq!(pack_coils(&[true, true]), &[0b_11]);
        assert_eq!(pack_coils(&[true; 8]), &[0b_1111_1111]);
        assert_eq!(pack_coils(&[true; 9]), &[255, 1]);
        assert_eq!(pack_coils(&[false; 8]), &[0]);
        assert_eq!(pack_coils(&[false; 9]), &[0, 0]);
    }

    #[test]
    fn test_unpack_bits() {
        assert_eq!(unpack_coils(&[], 0), &[]);
        assert_eq!(unpack_coils(&[0, 0], 0), &[]);
        assert_eq!(unpack_coils(&[0b1], 1), &[true]);
        assert_eq!(unpack_coils(&[0b01], 2), &[true, false]);
        assert_eq!(unpack_coils(&[0b10], 2), &[false, true]);
        assert_eq!(unpack_coils(&[0b101], 3), &[true, false, true]);
        assert_eq!(unpack_coils(&[0xff, 0b11], 10), &[true; 10]);
    }

    #[test]
    fn function_code_from_request() {
        use crate::frame::Request::*;
        assert_eq!(req_to_fn_code(&ReadCoils(0, 0)), 1);
        assert_eq!(req_to_fn_code(&ReadDiscreteInputs(0, 0)), 2);
        assert_eq!(req_to_fn_code(&WriteSingleCoil(0, true)), 5);
        assert_eq!(req_to_fn_code(&WriteMultipleCoils(0, vec![])), 0x0F);
        assert_eq!(req_to_fn_code(&ReadInputRegisters(0, 0)), 0x04);
        assert_eq!(req_to_fn_code(&ReadHoldingRegisters(0, 0)), 0x03);
        assert_eq!(req_to_fn_code(&WriteSingleRegister(0, 0)), 0x06);
        assert_eq!(req_to_fn_code(&WriteMultipleRegisters(0, vec![])), 0x10);
        assert_eq!(
            req_to_fn_code(&ReadWriteMultipleRegisters(0, 0, 0, vec![])),
            0x17
        );
        assert_eq!(req_to_fn_code(&Custom(88, vec![])), 88);
    }

    #[test]
    fn function_code_from_response() {
        use crate::frame::Response::*;
        assert_eq!(rsp_to_fn_code(&ReadCoils(vec![])), 1);
        assert_eq!(rsp_to_fn_code(&ReadDiscreteInputs(vec![])), 2);
        assert_eq!(rsp_to_fn_code(&WriteSingleCoil(0x0, false)), 5);
        assert_eq!(rsp_to_fn_code(&WriteMultipleCoils(0x0, 0x0)), 0x0F);
        assert_eq!(rsp_to_fn_code(&ReadInputRegisters(vec![])), 0x04);
        assert_eq!(rsp_to_fn_code(&ReadHoldingRegisters(vec![])), 0x03);
        assert_eq!(rsp_to_fn_code(&WriteSingleRegister(0, 0)), 0x06);
        assert_eq!(rsp_to_fn_code(&WriteMultipleRegisters(0, 0)), 0x10);
        assert_eq!(rsp_to_fn_code(&ReadWriteMultipleRegisters(vec![])), 0x17);
        assert_eq!(rsp_to_fn_code(&Custom(99, vec![])), 99);
    }

    #[test]
    fn exception_response_into_bytes() {
        let bytes: Bytes = ExceptionResponse {
            function: 0x03,
            exception: Exception::IllegalDataAddress,
        }
        .into();
        assert_eq!(bytes[0], 0x83);
        assert_eq!(bytes[1], 0x02);
    }

    #[test]
    fn exception_response_from_bytes() {
        assert!(ExceptionResponse::try_from(Bytes::from(vec![0x79, 0x02])).is_err());

        let bytes = Bytes::from(vec![0x83, 0x02]);
        let rsp = ExceptionResponse::try_from(bytes).unwrap();
        assert_eq!(
            rsp,
            ExceptionResponse {
                function: 0x03,
                exception: Exception::IllegalDataAddress,
            }
        );
    }

    #[test]
    fn pdu_into_bytes() {
        let req_pdu: Bytes = Request::ReadCoils(0x01, 5).into();
        let rsp_pdu: Bytes = Response::ReadCoils(vec![]).into();
        let ex_pdu: Bytes = ExceptionResponse {
            function: 0x03,
            exception: Exception::ServerDeviceFailure,
        }
        .into();

        assert_eq!(req_pdu[0], 0x01);
        assert_eq!(req_pdu[1], 0x00);
        assert_eq!(req_pdu[2], 0x01);
        assert_eq!(req_pdu[3], 0x00);
        assert_eq!(req_pdu[4], 0x05);

        assert_eq!(rsp_pdu[0], 0x01);
        assert_eq!(rsp_pdu[1], 0x00);

        assert_eq!(ex_pdu[0], 0x83);
        assert_eq!(ex_pdu[1], 0x04);

        let req_pdu: Bytes = Request::ReadHoldingRegisters(0x082B, 2).into();
        assert_eq!(req_pdu.len(), 5);
        assert_eq!(req_pdu[0], 0x03);
        assert_eq!(req_pdu[1], 0x08);
        assert_eq!(req_pdu[2], 0x2B);
        assert_eq!(req_pdu[3], 0x00);
        assert_eq!(req_pdu[4], 0x02);
    }

    #[test]
    fn pdu_with_a_lot_of_data_into_bytes() {
        let _req_pdu: Bytes = Request::WriteMultipleRegisters(0x01, vec![0; 80]).into();
        let _rsp_pdu: Bytes = Response::ReadInputRegisters(vec![0; 80]).into();
    }

    mod serialize_requests {

        use super::*;

        #[test]
        fn read_coils() {
            let bytes: Bytes = Request::ReadCoils(0x12, 4).into();
            assert_eq!(bytes[0], 1);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x12);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x04);
        }

        #[test]
        fn read_discrete_inputs() {
            let bytes: Bytes = Request::ReadDiscreteInputs(0x03, 19).into();
            assert_eq!(bytes[0], 2);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x03);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 19);
        }

        #[test]
        fn write_single_coil() {
            let bytes: Bytes = Request::WriteSingleCoil(0x1234, true).into();
            assert_eq!(bytes[0], 5);
            assert_eq!(bytes[1], 0x12);
            assert_eq!(bytes[2], 0x34);
            assert_eq!(bytes[3], 0xFF);
            assert_eq!(bytes[4], 0x00);
        }

        #[test]
        fn write_multiple_coils() {
            let states = vec![true, false, true, true];
            let bytes: Bytes = Request::WriteMultipleCoils(0x3311, states).into();
            assert_eq!(bytes[0], 0x0F);
            assert_eq!(bytes[1], 0x33);
            assert_eq!(bytes[2], 0x11);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x04);
            assert_eq!(bytes[5], 0x01);
            assert_eq!(bytes[6], 0b_0000_1101);
        }

        #[test]
        fn read_input_registers() {
            let bytes: Bytes = Request::ReadInputRegisters(0x09, 77).into();
            assert_eq!(bytes[0], 4);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x09);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x4D);
        }

        #[test]
        fn read_holding_registers() {
            let bytes: Bytes = Request::ReadHoldingRegisters(0x09, 77).into();
            assert_eq!(bytes[0], 3);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x09);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x4D);
        }

        #[test]
        fn write_single_register() {
            let bytes: Bytes = Request::WriteSingleRegister(0x07, 0xABCD).into();
            assert_eq!(bytes[0], 6);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x07);
            assert_eq!(bytes[3], 0xAB);
            assert_eq!(bytes[4], 0xCD);
        }

        #[test]
        fn write_multiple_registers() {
            let bytes: Bytes = Request::WriteMultipleRegisters(0x06, vec![0xABCD, 0xEF12]).into();

            // function code
            assert_eq!(bytes[0], 0x10);

            // write starting address
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x06);

            // quantity to write
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x02);

            // write byte count
            assert_eq!(bytes[5], 0x04);

            // values
            assert_eq!(bytes[6], 0xAB);
            assert_eq!(bytes[7], 0xCD);
            assert_eq!(bytes[8], 0xEF);
            assert_eq!(bytes[9], 0x12);
        }

        #[test]
        fn read_write_multiple_registers() {
            let data = vec![0xABCD, 0xEF12];
            let bytes: Bytes = Request::ReadWriteMultipleRegisters(0x05, 51, 0x03, data).into();

            // function code
            assert_eq!(bytes[0], 0x17);

            // read starting address
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x05);

            // quantity to read
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x33);

            // write starting address
            assert_eq!(bytes[5], 0x00);
            assert_eq!(bytes[6], 0x03);

            // quantity to write
            assert_eq!(bytes[7], 0x00);
            assert_eq!(bytes[8], 0x02);

            // write byte count
            assert_eq!(bytes[9], 0x04);

            // values
            assert_eq!(bytes[10], 0xAB);
            assert_eq!(bytes[11], 0xCD);
            assert_eq!(bytes[12], 0xEF);
            assert_eq!(bytes[13], 0x12);
        }

        #[test]
        fn custom() {
            let bytes: Bytes = Request::Custom(0x55, vec![0xCC, 0x88, 0xAA, 0xFF]).into();
            assert_eq!(bytes[0], 0x55);
            assert_eq!(bytes[1], 0xCC);
            assert_eq!(bytes[2], 0x88);
            assert_eq!(bytes[3], 0xAA);
            assert_eq!(bytes[4], 0xFF);
        }
    }

    mod deserialize_requests {

        use super::*;

        #[test]
        fn empty_request() {
            assert!(Request::try_from(Bytes::from(vec![])).is_err());
        }

        #[test]
        fn read_coils() {
            assert!(Request::try_from(Bytes::from(vec![0x01])).is_err());
            assert!(Request::try_from(Bytes::from(vec![0x01, 0x0, 0x0, 0x22])).is_err());

            let bytes = Bytes::from(vec![0x01, 0x00, 0x12, 0x0, 0x4]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::ReadCoils(0x12, 4));
        }

        #[test]
        fn read_discrete_inputs() {
            let bytes = Bytes::from(vec![2, 0x00, 0x03, 0x00, 19]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::ReadDiscreteInputs(0x03, 19));
        }

        #[test]
        fn write_single_coil() {
            let bytes = Bytes::from(vec![5, 0x12, 0x34, 0xFF, 0x00]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::WriteSingleCoil(0x1234, true));
        }

        #[test]
        fn write_multiple_coils() {
            assert!(Request::try_from(Bytes::from(vec![
                0x0F,
                0x33,
                0x11,
                0x00,
                0x04,
                0x02,
                0b_0000_1101,
            ]))
            .is_err());

            let bytes = Bytes::from(vec![0x0F, 0x33, 0x11, 0x00, 0x04, 0x01, 0b_0000_1101]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(
                req,
                Request::WriteMultipleCoils(0x3311, vec![true, false, true, true])
            );
        }

        #[test]
        fn read_input_registers() {
            let bytes = Bytes::from(vec![4, 0x00, 0x09, 0x00, 0x4D]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::ReadInputRegisters(0x09, 77));
        }

        #[test]
        fn read_holding_registers() {
            let bytes = Bytes::from(vec![3, 0x00, 0x09, 0x00, 0x4D]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::ReadHoldingRegisters(0x09, 77));
        }

        #[test]
        fn write_single_register() {
            let bytes = Bytes::from(vec![6, 0x00, 0x07, 0xAB, 0xCD]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::WriteSingleRegister(0x07, 0xABCD));
        }

        #[test]
        fn write_multiple_registers() {
            assert!(Request::try_from(Bytes::from(vec![
                0x10, 0x00, 0x06, 0x00, 0x02, 0x05, 0xAB, 0xCD, 0xEF, 0x12,
            ]))
            .is_err());

            let bytes = Bytes::from(vec![
                0x10, 0x00, 0x06, 0x00, 0x02, 0x04, 0xAB, 0xCD, 0xEF, 0x12,
            ]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(
                req,
                Request::WriteMultipleRegisters(0x06, vec![0xABCD, 0xEF12])
            );
        }

        #[test]
        fn read_write_multiple_registers() {
            assert!(Request::try_from(Bytes::from(vec![
                0x17, 0x00, 0x05, 0x00, 0x33, 0x00, 0x03, 0x00, 0x02, 0x05, 0xAB, 0xCD, 0xEF, 0x12,
            ]))
            .is_err());
            let bytes = Bytes::from(vec![
                0x17, 0x00, 0x05, 0x00, 0x33, 0x00, 0x03, 0x00, 0x02, 0x04, 0xAB, 0xCD, 0xEF, 0x12,
            ]);
            let req = Request::try_from(bytes).unwrap();
            let data = vec![0xABCD, 0xEF12];
            assert_eq!(
                req,
                Request::ReadWriteMultipleRegisters(0x05, 51, 0x03, data)
            );
        }

        #[test]
        fn custom() {
            let bytes = Bytes::from(vec![0x55, 0xCC, 0x88, 0xAA, 0xFF]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::Custom(0x55, vec![0xCC, 0x88, 0xAA, 0xFF]));
        }
    }

    mod serialize_responses {

        use super::*;

        #[test]
        fn read_coils() {
            let bytes: Bytes = Response::ReadCoils(vec![true, false, false, true, false]).into();
            assert_eq!(bytes[0], 1);
            assert_eq!(bytes[1], 1);
            assert_eq!(bytes[2], 0b_0000_1001);
        }

        #[test]
        fn read_discrete_inputs() {
            let bytes: Bytes = Response::ReadDiscreteInputs(vec![true, false, true, true]).into();
            assert_eq!(bytes[0], 2);
            assert_eq!(bytes[1], 1);
            assert_eq!(bytes[2], 0b_0000_1101);
        }

        #[test]
        fn write_single_coil() {
            let bytes: Bytes = Response::WriteSingleCoil(0x33, true).into();
            assert_eq!(bytes[0], 5);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x33);
            assert_eq!(bytes[3], 0xFF);
            assert_eq!(bytes[4], 0x00);
        }

        #[test]
        fn write_multiple_coils() {
            let bytes: Bytes = Response::WriteMultipleCoils(0x3311, 5).into();
            assert_eq!(bytes[0], 0x0F);
            assert_eq!(bytes[1], 0x33);
            assert_eq!(bytes[2], 0x11);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x05);
        }

        #[test]
        fn read_input_registers() {
            let bytes: Bytes = Response::ReadInputRegisters(vec![0xAA00, 0xCCBB, 0xEEDD]).into();
            assert_eq!(bytes[0], 4);
            assert_eq!(bytes[1], 0x06);
            assert_eq!(bytes[2], 0xAA);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0xCC);
            assert_eq!(bytes[5], 0xBB);
            assert_eq!(bytes[6], 0xEE);
            assert_eq!(bytes[7], 0xDD);
        }

        #[test]
        fn read_holding_registers() {
            let bytes: Bytes = Response::ReadHoldingRegisters(vec![0xAA00, 0x1111]).into();
            assert_eq!(bytes[0], 3);
            assert_eq!(bytes[1], 0x04);
            assert_eq!(bytes[2], 0xAA);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x11);
            assert_eq!(bytes[5], 0x11);
        }

        #[test]
        fn write_single_register() {
            let bytes: Bytes = Response::WriteSingleRegister(0x07, 0xABCD).into();
            assert_eq!(bytes[0], 6);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x07);
            assert_eq!(bytes[3], 0xAB);
            assert_eq!(bytes[4], 0xCD);
        }

        #[test]
        fn write_multiple_registers() {
            let bytes: Bytes = Response::WriteMultipleRegisters(0x06, 2).into();
            assert_eq!(bytes[0], 0x10);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x06);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x02);
        }

        #[test]
        fn read_write_multiple_registers() {
            let bytes: Bytes = Response::ReadWriteMultipleRegisters(vec![0x1234]).into();
            assert_eq!(bytes[0], 0x17);
            assert_eq!(bytes[1], 0x02);
            assert_eq!(bytes[2], 0x12);
            assert_eq!(bytes[3], 0x34);
        }

        #[test]
        fn custom() {
            let bytes: Bytes = Response::Custom(0x55, vec![0xCC, 0x88, 0xAA, 0xFF]).into();
            assert_eq!(bytes[0], 0x55);
            assert_eq!(bytes[1], 0xCC);
            assert_eq!(bytes[2], 0x88);
            assert_eq!(bytes[3], 0xAA);
            assert_eq!(bytes[4], 0xFF);
        }
    }

    mod deserialize_responses {

        use super::*;

        #[test]
        fn read_coils() {
            let bytes = Bytes::from(vec![1, 1, 0b_0000_1001]);
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(
                rsp,
                Response::ReadCoils(vec![true, false, false, true, false, false, false, false])
            );
        }

        #[test]
        fn read_discrete_inputs() {
            let bytes = Bytes::from(vec![2, 1, 0b_0000_1001]);
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(
                rsp,
                Response::ReadDiscreteInputs(vec![
                    true, false, false, true, false, false, false, false,
                ],)
            );
        }

        #[test]
        fn write_single_coil() {
            let bytes = Bytes::from(vec![5, 0x00, 0x33, 0xFF, 0x00]);
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(rsp, Response::WriteSingleCoil(0x33, true  ));
        }

        #[test]
        fn write_multiple_coils() {
            let bytes = Bytes::from(vec![0x0F, 0x33, 0x11, 0x00, 0x05]);
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(rsp, Response::WriteMultipleCoils(0x3311, 5));
        }

        #[test]
        fn read_input_registers() {
            let bytes = Bytes::from(vec![4, 0x06, 0xAA, 0x00, 0xCC, 0xBB, 0xEE, 0xDD]);
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(
                rsp,
                Response::ReadInputRegisters(vec![0xAA00, 0xCCBB, 0xEEDD])
            );
        }

        #[test]
        fn read_holding_registers() {
            let bytes = Bytes::from(vec![3, 0x04, 0xAA, 0x00, 0x11, 0x11]);
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(rsp, Response::ReadHoldingRegisters(vec![0xAA00, 0x1111]));
        }

        #[test]
        fn write_single_register() {
            let bytes = Bytes::from(vec![6, 0x00, 0x07, 0xAB, 0xCD]);
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(rsp, Response::WriteSingleRegister(0x07, 0xABCD));
        }

        #[test]
        fn write_multiple_registers() {
            let bytes = Bytes::from(vec![0x10, 0x00, 0x06, 0x00, 0x02]);
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(rsp, Response::WriteMultipleRegisters(0x06, 2));
        }

        #[test]
        fn read_write_multiple_registers() {
            let bytes = Bytes::from(vec![0x17, 0x02, 0x12, 0x34]);
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(rsp, Response::ReadWriteMultipleRegisters(vec![0x1234]));
        }

        #[test]
        fn custom() {
            let bytes = Bytes::from(vec![0x55, 0xCC, 0x88, 0xAA, 0xFF]);
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(rsp, Response::Custom(0x55, vec![0xCC, 0x88, 0xAA, 0xFF]));
        }
    }
}
