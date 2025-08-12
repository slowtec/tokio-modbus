// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{
    convert::TryFrom,
    io::{self, BufRead as _, Cursor, Error, ErrorKind},
};

use byteorder::{BigEndian, ReadBytesExt as _};

use crate::{
    bytes::{Buf as _, Bytes},
    frame::{
        Coil, ConformityLevel, DeviceIdObject, ReadCode, RequestPdu, ResponsePdu,
        MEI_TYPE_READ_DEVICE_IDENTIFICATION,
    },
    ExceptionCode, ExceptionResponse, FunctionCode, Request, Response,
};

#[cfg(feature = "rtu")]
pub(crate) mod rtu;

#[cfg(feature = "tcp")]
pub(crate) mod tcp;

/// Maximum request/response PDU size.
///
/// As defined by the spec for both RTU and TCP.
const MAX_PDU_SIZE: usize = 253;

#[cfg(any(test, feature = "rtu", feature = "tcp"))]
#[allow(clippy::cast_possible_truncation)]
fn u16_len(len: usize) -> u16 {
    // This type conversion should always be safe, because either
    // the caller is responsible to pass a valid usize or the
    // possible values are limited by the protocol.
    debug_assert!(len <= u16::MAX.into());
    len as u16
}

#[cfg(any(test, feature = "rtu", feature = "tcp"))]
#[allow(clippy::cast_possible_truncation)]
fn u8_len(len: usize) -> u8 {
    // This type conversion should always be safe, because either
    // the caller is responsible to pass a valid usize or the
    // possible values are limited by the protocol.
    debug_assert!(len <= u8::MAX.into());
    len as u8
}

#[cfg(any(test, feature = "rtu", feature = "tcp"))]
fn encode_request_pdu(buf: &mut crate::bytes::BytesMut, request: &Request<'_>) {
    use crate::{bytes::BufMut as _, frame::Request::*};
    buf.put_u8(request.function_code().value());
    match request {
        ReadCoils(address, quantity)
        | ReadDiscreteInputs(address, quantity)
        | ReadInputRegisters(address, quantity)
        | ReadHoldingRegisters(address, quantity) => {
            buf.put_u16(*address);
            buf.put_u16(*quantity);
        }
        WriteSingleCoil(address, state) => {
            buf.put_u16(*address);
            buf.put_u16(bool_to_coil(*state));
        }
        WriteMultipleCoils(address, coils) => {
            buf.put_u16(*address);
            buf.put_u16(u16_len(coils.len()));
            buf.put_u8(u8_len(packed_coils_size(coils)));
            encode_packed_coils(buf, coils);
        }
        WriteSingleRegister(address, word) => {
            buf.put_u16(*address);
            buf.put_u16(*word);
        }
        WriteMultipleRegisters(address, words) => {
            buf.put_u16(*address);
            let len = words.len();
            buf.put_u16(u16_len(len));
            buf.put_u8(u8_len(len * 2));
            for w in words.as_ref() {
                buf.put_u16(*w);
            }
        }
        ReportServerId => {}
        MaskWriteRegister(address, and_mask, or_mask) => {
            buf.put_u16(*address);
            buf.put_u16(*and_mask);
            buf.put_u16(*or_mask);
        }
        ReadWriteMultipleRegisters(read_address, quantity, write_address, words) => {
            buf.put_u16(*read_address);
            buf.put_u16(*quantity);
            buf.put_u16(*write_address);
            let len = words.len();
            buf.put_u16(u16_len(len));
            buf.put_u8(u8_len(len * 2));
            for w in words.as_ref() {
                buf.put_u16(*w);
            }
        }
        ReadDeviceIdentification(read_code, object_id) => {
            buf.put_u8(MEI_TYPE_READ_DEVICE_IDENTIFICATION);
            buf.put_u8(read_code.value());
            buf.put_u8(*object_id);
        }
        Custom(_, custom_data) => {
            buf.put_slice(custom_data.as_ref());
        }
    }
}

#[cfg(any(test, feature = "server"))]
fn encode_response_pdu(buf: &mut crate::bytes::BytesMut, response: &Response) {
    use crate::{bytes::BufMut as _, frame::Response::*};
    buf.put_u8(response.function_code().value());
    match response {
        ReadCoils(coils) | ReadDiscreteInputs(coils) => {
            buf.put_u8(u8_len(packed_coils_size(coils)));
            encode_packed_coils(buf, coils);
        }
        ReadInputRegisters(registers)
        | ReadHoldingRegisters(registers)
        | ReadWriteMultipleRegisters(registers) => {
            buf.put_u8(u8_len(registers.len() * 2));
            for r in registers {
                buf.put_u16(*r);
            }
        }
        WriteSingleCoil(address, state) => {
            buf.put_u16(*address);
            buf.put_u16(bool_to_coil(*state));
        }
        WriteMultipleCoils(address, quantity) | WriteMultipleRegisters(address, quantity) => {
            buf.put_u16(*address);
            buf.put_u16(*quantity);
        }
        ReportServerId(server_id, run_indication, additional_data) => {
            buf.put_u8(2 + u8_len(additional_data.len()));
            buf.put_u8(*server_id);
            buf.put_u8(if *run_indication { 0xFF } else { 0x00 });
            buf.put_slice(additional_data);
        }
        WriteSingleRegister(address, word) => {
            buf.put_u16(*address);
            buf.put_u16(*word);
        }
        MaskWriteRegister(address, and_mask, or_mask) => {
            buf.put_u16(*address);
            buf.put_u16(*and_mask);
            buf.put_u16(*or_mask);
        }
        ReadDeviceIdentification(
            read_code,
            conformity_level,
            more_follows,
            next_object_id,
            device_id_objects,
        ) => {
            buf.put_u8(MEI_TYPE_READ_DEVICE_IDENTIFICATION);
            buf.put_u8(read_code.value());
            buf.put_u8(conformity_level.value());
            buf.put_u8(if *more_follows { 0xff } else { 0x00 });
            buf.put_u8(*next_object_id);
            buf.put_u8(device_id_objects.len() as u8); // response_pdu_size validates the length
            for dio in device_id_objects {
                buf.put_u8(dio.id);
                buf.put_u8(dio.value.len() as u8); // response_pdu_size validates the length
                buf.put_slice(&dio.value);
            }
        }
        Custom(_, custom_data) => {
            buf.put_slice(custom_data);
        }
    }
}

#[cfg(any(test, feature = "server"))]
fn encode_exception_response_pdu(buf: &mut crate::bytes::BytesMut, response: ExceptionResponse) {
    use crate::bytes::BufMut as _;
    debug_assert!(response.function.value() < 0x80);
    buf.put_u8(response.function.value() + 0x80);
    buf.put_u8(response.exception.into());
}

#[cfg(feature = "server")]
fn encode_response_result_pdu(
    buf: &mut crate::bytes::BytesMut,
    res: &Result<Response, ExceptionResponse>,
) {
    match res {
        Ok(response) => encode_response_pdu(buf, response),
        Err(response) => encode_exception_response_pdu(buf, *response),
    }
}

fn read_u16_be(reader: &mut impl io::Read) -> io::Result<u16> {
    reader.read_u16::<BigEndian>()
}

// Only needed for requests with a dynamic payload size.
fn check_request_pdu_size(pdu_size: usize) -> io::Result<()> {
    if pdu_size > MAX_PDU_SIZE {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "request PDU size exceeded",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // TODO
fn decode_request_pdu_bytes(bytes: &Bytes) -> io::Result<Request<'static>> {
    use crate::frame::Request::*;
    let pdu_size = bytes.len();
    let rdr = &mut Cursor::new(&bytes);
    let fn_code = rdr.read_u8()?;
    let req = match fn_code {
        0x01 => ReadCoils(read_u16_be(rdr)?, read_u16_be(rdr)?),
        0x02 => ReadDiscreteInputs(read_u16_be(rdr)?, read_u16_be(rdr)?),
        0x05 => WriteSingleCoil(read_u16_be(rdr)?, coil_to_bool(read_u16_be(rdr)?)?),
        0x0F => {
            check_request_pdu_size(pdu_size)?;
            let address = read_u16_be(rdr)?;
            let quantity = read_u16_be(rdr)?;
            let byte_count = usize::from(rdr.read_u8()?);
            if bytes.len() < 6 + byte_count {
                return Err(io::Error::new(ErrorKind::InvalidData, "too short"));
            }
            rdr.consume(byte_count);
            let packed_coils = &bytes[6..6 + byte_count];
            WriteMultipleCoils(address, decode_packed_coils(packed_coils, quantity).into())
        }
        0x04 => ReadInputRegisters(read_u16_be(rdr)?, read_u16_be(rdr)?),
        0x03 => ReadHoldingRegisters(read_u16_be(rdr)?, read_u16_be(rdr)?),
        0x06 => WriteSingleRegister(read_u16_be(rdr)?, read_u16_be(rdr)?),
        0x10 => {
            check_request_pdu_size(pdu_size)?;
            let address = read_u16_be(rdr)?;
            let quantity = read_u16_be(rdr)?;
            let byte_count = rdr.read_u8()?;
            if u16::from(byte_count) != quantity * 2 {
                return Err(io::Error::new(ErrorKind::InvalidData, "invalid quantity"));
            }
            let mut data = Vec::with_capacity(quantity.into());
            for _ in 0..quantity {
                data.push(read_u16_be(rdr)?);
            }
            WriteMultipleRegisters(address, data.into())
        }
        0x11 => ReportServerId,
        0x16 => {
            let address = read_u16_be(rdr)?;
            let and_mask = read_u16_be(rdr)?;
            let or_mask = read_u16_be(rdr)?;
            MaskWriteRegister(address, and_mask, or_mask)
        }
        0x17 => {
            check_request_pdu_size(pdu_size)?;
            let read_address = read_u16_be(rdr)?;
            let read_quantity = read_u16_be(rdr)?;
            let write_address = read_u16_be(rdr)?;
            let write_quantity = read_u16_be(rdr)?;
            let write_count = rdr.read_u8()?;
            if u16::from(write_count) != write_quantity * 2 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid write quantity",
                ));
            }
            let mut data = Vec::with_capacity(write_quantity.into());
            for _ in 0..write_quantity {
                data.push(read_u16_be(rdr)?);
            }
            ReadWriteMultipleRegisters(read_address, read_quantity, write_address, data.into())
        }
        0x2b if rdr.read_u8()? == MEI_TYPE_READ_DEVICE_IDENTIFICATION => {
            check_request_pdu_size(pdu_size)?;
            let Some(read_device_id_code) = ReadCode::try_from_value(rdr.read_u8()?) else {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid Read device ID code",
                ));
            };
            let object_id = rdr.read_u8()?;
            ReadDeviceIdentification(read_device_id_code, object_id)
        }
        fn_code if fn_code < 0x80 => {
            // Consume all remaining bytes as custom data.
            return Ok(Custom(fn_code, bytes[1..].to_vec().into()));
        }
        fn_code => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid function code: 0x{fn_code:02X}"),
            ));
        }
    };
    // Verify that all data has been consumed and decoded.
    if rdr.has_remaining() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "undecoded request data",
        ));
    }
    Ok(req)
}

impl TryFrom<Bytes> for Request<'static> {
    type Error = Error;

    fn try_from(pdu_bytes: Bytes) -> Result<Self, Self::Error> {
        decode_request_pdu_bytes(&pdu_bytes)
    }
}

impl TryFrom<Bytes> for RequestPdu<'static> {
    type Error = Error;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        let pdu = Request::try_from(bytes)?.into();
        Ok(pdu)
    }
}

// Only needed for responses with a dynamic payload size.
fn check_response_pdu_size(pdu_size: usize) -> io::Result<()> {
    if pdu_size > MAX_PDU_SIZE {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "response PDU size exceeded",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // TODO
fn decode_response_pdu_bytes(bytes: Bytes) -> io::Result<Response> {
    use crate::frame::Response::*;
    let pdu_size = bytes.len();
    let rdr = &mut Cursor::new(&bytes);
    let fn_code = rdr.read_u8()?;
    let response = match fn_code {
        0x01 => {
            check_response_pdu_size(pdu_size)?;
            let byte_count = rdr.read_u8()?;
            if bytes.len() < 2 + usize::from(byte_count) {
                return Err(io::Error::new(ErrorKind::InvalidData, "too short"));
            }
            let packed_coils = &bytes[2..2 + usize::from(byte_count)];
            rdr.consume(byte_count.into());
            // Here we have not information about the exact requested quantity so we just
            // unpack the whole byte.
            let quantity = u16::from(byte_count) * 8;
            ReadCoils(decode_packed_coils(packed_coils, quantity))
        }
        0x02 => {
            check_response_pdu_size(pdu_size)?;
            let byte_count = rdr.read_u8()?;
            if bytes.len() < 2 + usize::from(byte_count) {
                return Err(io::Error::new(ErrorKind::InvalidData, "too short"));
            }
            let packed_coils = &bytes[2..2 + usize::from(byte_count)];
            rdr.consume(byte_count.into());
            // Here we have no information about the exact requested quantity so we just
            // unpack the whole byte.
            let quantity = u16::from(byte_count) * 8;
            ReadDiscreteInputs(decode_packed_coils(packed_coils, quantity))
        }
        0x05 => WriteSingleCoil(read_u16_be(rdr)?, coil_to_bool(read_u16_be(rdr)?)?),
        0x0F => WriteMultipleCoils(read_u16_be(rdr)?, read_u16_be(rdr)?),
        0x04 => {
            check_response_pdu_size(pdu_size)?;
            let byte_count = rdr.read_u8()?;
            if byte_count % 2 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid quantity",
                ));
            }
            let quantity = byte_count / 2;
            let mut data = Vec::with_capacity(quantity.into());
            for _ in 0..quantity {
                data.push(read_u16_be(rdr)?);
            }
            ReadInputRegisters(data)
        }
        0x03 => {
            check_response_pdu_size(pdu_size)?;
            let byte_count = rdr.read_u8()?;
            if byte_count % 2 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid quantity",
                ));
            }
            let quantity = byte_count / 2;
            let mut data = Vec::with_capacity(quantity.into());
            for _ in 0..quantity {
                data.push(read_u16_be(rdr)?);
            }
            ReadHoldingRegisters(data)
        }
        0x06 => WriteSingleRegister(read_u16_be(rdr)?, read_u16_be(rdr)?),
        0x10 => WriteMultipleRegisters(read_u16_be(rdr)?, read_u16_be(rdr)?),
        0x11 => {
            check_response_pdu_size(pdu_size)?;
            let byte_count = rdr.read_u8()?;
            if byte_count < 2 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "too short"));
            }
            let data_len = (byte_count - 2).into();
            let server_id = rdr.read_u8()?;
            let run_indication_status = match rdr.read_u8()? {
                0x00 => false,
                0xFF => true,
                status => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("invalid run indication status: 0x{status:02X}"),
                    ));
                }
            };
            let mut data = Vec::with_capacity(data_len);
            for _ in 0..data_len {
                data.push(rdr.read_u8()?);
            }
            ReportServerId(server_id, run_indication_status, data)
        }
        0x16 => {
            let address = read_u16_be(rdr)?;
            let and_mask = read_u16_be(rdr)?;
            let or_mask = read_u16_be(rdr)?;
            MaskWriteRegister(address, and_mask, or_mask)
        }
        0x17 => {
            check_response_pdu_size(pdu_size)?;
            let byte_count = rdr.read_u8()?;
            if byte_count % 2 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid quantity",
                ));
            }
            let quantity = byte_count / 2;
            let mut data = Vec::with_capacity(quantity.into());
            for _ in 0..quantity {
                data.push(read_u16_be(rdr)?);
            }
            ReadWriteMultipleRegisters(data)
        }
        0x2b if rdr.read_u8()? == MEI_TYPE_READ_DEVICE_IDENTIFICATION => {
            check_response_pdu_size(pdu_size)?;
            let Some(read_device_id_code) = ReadCode::try_from_value(rdr.read_u8()?) else {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid Read device ID code",
                ));
            };
            let Some(conformity_level) = ConformityLevel::try_from_value(rdr.read_u8()?) else {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid conformity level",
                ));
            };
            let more_follows = rdr.read_u8()? == 0xff;
            let next_object_id = rdr.read_u8()?;
            let count = rdr.read_u8()?;
            let mut objects = Vec::with_capacity(count.into());
            for _ in 0..count {
                let id = rdr.read_u8()?;
                let len: usize = rdr.read_u8()?.into();

                let position = rdr.position() as usize;
                let bytes = rdr.get_ref();

                if position + len > bytes.len() {
                    return Err(Error::new(ErrorKind::InvalidData, "Invalid object length"));
                }

                let value = bytes.slice(position..position + len);
                rdr.set_position((position + len) as u64);

                objects.push(DeviceIdObject { id, value });
            }
            ReadDeviceIdentification(
                read_device_id_code,
                conformity_level,
                more_follows,
                next_object_id,
                objects,
            )
        }
        _ => {
            // Consume all remaining bytes as custom data.
            let mut bytes = bytes;
            return Ok(Custom(fn_code, bytes.split_off(1)));
        }
    };
    // Verify that all data has been consumed and decoded.
    if rdr.has_remaining() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "undecoded response data",
        ));
    }
    Ok(response)
}

impl TryFrom<Bytes> for Response {
    type Error = Error;

    fn try_from(pdu_bytes: Bytes) -> Result<Self, Self::Error> {
        decode_response_pdu_bytes(pdu_bytes)
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
        let exception = ExceptionCode::new(rdr.read_u8()?);
        Ok(ExceptionResponse {
            function: FunctionCode::new(function),
            exception,
        })
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

#[cfg(any(test, feature = "rtu", feature = "tcp"))]
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

#[cfg(any(test, feature = "rtu", feature = "tcp"))]
fn packed_coils_size(coils: &[Coil]) -> usize {
    coils.len().div_ceil(8)
}

#[cfg(any(test, feature = "rtu", feature = "tcp"))]
fn encode_packed_coils(buf: &mut crate::bytes::BytesMut, coils: &[Coil]) -> usize {
    let packed_coils_size = packed_coils_size(coils);
    let offset = buf.len();
    buf.resize(offset + packed_coils_size, 0);
    let buf = &mut buf[offset..];
    for (i, b) in coils.iter().enumerate() {
        let v = u8::from(*b); // 0 or 1
        buf[i / 8] |= v << (i % 8);
    }
    packed_coils_size
}

fn decode_packed_coils(bytes: &[u8], count: u16) -> Vec<Coil> {
    let mut res = Vec::with_capacity(count.into());
    for i in 0usize..count.into() {
        res.push((bytes[i / 8] >> (i % 8)) & 0b1 > 0);
    }
    res
}

#[cfg(any(feature = "rtu", feature = "tcp"))]
fn request_pdu_size(request: &Request<'_>) -> io::Result<usize> {
    use crate::frame::Request::*;
    let size = match request {
        ReadCoils(_, _)
        | ReadDiscreteInputs(_, _)
        | ReadInputRegisters(_, _)
        | ReadHoldingRegisters(_, _)
        | WriteSingleRegister(_, _)
        | WriteSingleCoil(_, _) => 5,
        WriteMultipleCoils(_, coils) => 6 + packed_coils_size(coils),
        WriteMultipleRegisters(_, data) => 6 + data.len() * 2,
        ReportServerId => 1,
        MaskWriteRegister(_, _, _) => 7,
        ReadWriteMultipleRegisters(_, _, _, data) => 10 + data.len() * 2,
        ReadDeviceIdentification(_, _) => 4,
        Custom(_, data) => 1 + data.len(),
    };
    if size > MAX_PDU_SIZE {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "request PDU size exceeded",
        ));
    }
    Ok(size)
}

#[cfg(feature = "server")]
fn response_pdu_size(response: &Response) -> io::Result<usize> {
    use crate::frame::Response::*;
    let size = match response {
        ReadCoils(coils) | ReadDiscreteInputs(coils) => 2 + packed_coils_size(coils),
        WriteSingleCoil(_, _)
        | WriteMultipleCoils(_, _)
        | WriteMultipleRegisters(_, _)
        | WriteSingleRegister(_, _) => 5,
        ReadInputRegisters(data)
        | ReadHoldingRegisters(data)
        | ReadWriteMultipleRegisters(data) => 2 + data.len() * 2,
        ReportServerId(_, _, data) => 3 + data.len(),
        MaskWriteRegister(_, _, _) => 7,
        ReadDeviceIdentification(_, _, _, _, device_id_objects) => {
            // 7-byte fixed header: function code, MEI type, device ID code,
            // conformity level, more follows flag, next object ID, and object count.
            //
            // Each object adds 2 bytes overhead (object ID + length) plus its value length.
            //
            // This calculation and the subsequent size check ensure the total length
            // fits within the maximum PDU size, implicitly limiting the number of objects
            // so it always fits in a u8.
            7 + device_id_objects
                .iter()
                .map(|o| 2 + o.value.len())
                .sum::<usize>()
        }
        Custom(_, data) => 1 + data.len(),
    };
    if size > MAX_PDU_SIZE {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "response PDU size exceeded",
        ));
    }
    Ok(size)
}

#[cfg(feature = "server")]
fn response_result_pdu_size(res: &Result<Response, ExceptionResponse>) -> io::Result<usize> {
    match res {
        Ok(response) => response_pdu_size(response),
        Err(_) => Ok(2),
    }
}

#[cfg(test)]
mod tests {

    use std::borrow::Cow;

    use crate::bytes::BytesMut;

    use super::*;

    fn encode_request_pdu_to_bytes(request: &Request<'_>) -> Bytes {
        let mut buf = BytesMut::new();
        encode_request_pdu(&mut buf, request);
        buf.freeze()
    }

    fn encode_response_pdu_to_bytes(response: &Response) -> Bytes {
        let mut buf = BytesMut::new();
        encode_response_pdu(&mut buf, response);
        buf.freeze()
    }

    fn encode_exception_response_pdu_to_bytes(response: ExceptionResponse) -> Bytes {
        let mut buf = BytesMut::new();
        encode_exception_response_pdu(&mut buf, response);
        buf.freeze()
    }

    fn encode_packed_coils_to_bytes(coils: &[Coil]) -> Bytes {
        let mut buf = BytesMut::new();
        encode_packed_coils(&mut buf, coils);
        buf.freeze()
    }

    #[test]
    fn convert_bool_to_coil() {
        assert_eq!(bool_to_coil(true), 0xFF00);
        assert_eq!(bool_to_coil(false), 0x0000);
    }

    #[test]
    fn convert_coil_to_bool() {
        assert!(coil_to_bool(0xFF00).unwrap());
        assert!(!coil_to_bool(0x0000).unwrap());
    }

    #[test]
    fn convert_booleans_to_bytes() {
        assert_eq!(&encode_packed_coils_to_bytes(&[])[..], &[]);
        assert_eq!(&encode_packed_coils_to_bytes(&[true])[..], &[0b1]);
        assert_eq!(&encode_packed_coils_to_bytes(&[false])[..], &[0b0]);
        assert_eq!(&encode_packed_coils_to_bytes(&[true, false])[..], &[0b_01]);
        assert_eq!(&encode_packed_coils_to_bytes(&[false, true])[..], &[0b_10]);
        assert_eq!(&encode_packed_coils_to_bytes(&[true, true])[..], &[0b_11]);
        assert_eq!(
            &encode_packed_coils_to_bytes(&[true; 8])[..],
            &[0b_1111_1111]
        );
        assert_eq!(&encode_packed_coils_to_bytes(&[true; 9])[..], &[255, 1]);
        assert_eq!(&encode_packed_coils_to_bytes(&[false; 8])[..], &[0]);
        assert_eq!(&encode_packed_coils_to_bytes(&[false; 9])[..], &[0, 0]);
    }

    #[test]
    fn test_unpack_bits() {
        assert_eq!(decode_packed_coils(&[], 0), &[]);
        assert_eq!(decode_packed_coils(&[0, 0], 0), &[]);
        assert_eq!(decode_packed_coils(&[0b1], 1), &[true]);
        assert_eq!(decode_packed_coils(&[0b01], 2), &[true, false]);
        assert_eq!(decode_packed_coils(&[0b10], 2), &[false, true]);
        assert_eq!(decode_packed_coils(&[0b101], 3), &[true, false, true]);
        assert_eq!(decode_packed_coils(&[0xff, 0b11], 10), &[true; 10]);
    }

    #[test]
    fn exception_response_into_bytes() {
        let bytes = encode_exception_response_pdu_to_bytes(ExceptionResponse {
            function: FunctionCode::ReadHoldingRegisters,
            exception: ExceptionCode::IllegalDataAddress,
        });
        assert_eq!(bytes[0], 0x83);
        assert_eq!(bytes[1], 0x02);
    }

    #[test]
    fn exception_response_from_bytes() {
        assert!(ExceptionResponse::try_from(Bytes::from(vec![0x79, 0x02])).is_err());

        let bytes = Bytes::from(vec![0x83, 0x02]);
        let response = ExceptionResponse::try_from(bytes).unwrap();
        assert_eq!(
            response,
            ExceptionResponse {
                function: FunctionCode::ReadHoldingRegisters,
                exception: ExceptionCode::IllegalDataAddress,
            }
        );
    }

    #[test]
    fn pdu_into_bytes() {
        let req_pdu = encode_request_pdu_to_bytes(&Request::ReadCoils(0x01, 5));
        let response_pdu = encode_response_pdu_to_bytes(&Response::ReadCoils(vec![]));
        let ex_pdu = encode_exception_response_pdu_to_bytes(ExceptionResponse {
            function: FunctionCode::ReadHoldingRegisters,
            exception: ExceptionCode::ServerDeviceFailure,
        });

        assert_eq!(req_pdu[0], 0x01);
        assert_eq!(req_pdu[1], 0x00);
        assert_eq!(req_pdu[2], 0x01);
        assert_eq!(req_pdu[3], 0x00);
        assert_eq!(req_pdu[4], 0x05);

        assert_eq!(response_pdu[0], 0x01);
        assert_eq!(response_pdu[1], 0x00);

        assert_eq!(ex_pdu[0], 0x83);
        assert_eq!(ex_pdu[1], 0x04);

        let req_pdu = encode_request_pdu_to_bytes(&Request::ReadHoldingRegisters(0x082B, 2));
        assert_eq!(req_pdu.len(), 5);
        assert_eq!(req_pdu[0], 0x03);
        assert_eq!(req_pdu[1], 0x08);
        assert_eq!(req_pdu[2], 0x2B);
        assert_eq!(req_pdu[3], 0x00);
        assert_eq!(req_pdu[4], 0x02);
    }

    #[test]
    fn pdu_with_a_lot_of_data_into_bytes() {
        let _req_pdu = encode_request_pdu_to_bytes(&Request::WriteMultipleRegisters(
            0x01,
            Cow::Borrowed(&[0; 80]),
        ));
        let _response_pdu =
            encode_response_pdu_to_bytes(&Response::ReadInputRegisters(vec![0; 80]));
    }

    mod serialize_requests {

        use super::*;

        #[test]
        fn read_coils() {
            let bytes = encode_request_pdu_to_bytes(&Request::ReadCoils(0x12, 4));
            assert_eq!(bytes[0], 1);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x12);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x04);
        }

        #[test]
        fn read_discrete_inputs() {
            let bytes = encode_request_pdu_to_bytes(&Request::ReadDiscreteInputs(0x03, 19));
            assert_eq!(bytes[0], 2);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x03);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 19);
        }

        #[test]
        fn write_single_coil() {
            let bytes = encode_request_pdu_to_bytes(&Request::WriteSingleCoil(0x1234, true));
            assert_eq!(bytes[0], 5);
            assert_eq!(bytes[1], 0x12);
            assert_eq!(bytes[2], 0x34);
            assert_eq!(bytes[3], 0xFF);
            assert_eq!(bytes[4], 0x00);
        }

        #[test]
        fn write_multiple_coils() {
            let states = [true, false, true, true];
            let bytes = encode_request_pdu_to_bytes(&Request::WriteMultipleCoils(
                0x3311,
                Cow::Borrowed(&states),
            ));
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
            let bytes = encode_request_pdu_to_bytes(&Request::ReadInputRegisters(0x09, 77));
            assert_eq!(bytes[0], 4);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x09);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x4D);
        }

        #[test]
        fn read_holding_registers() {
            let bytes = encode_request_pdu_to_bytes(&Request::ReadHoldingRegisters(0x09, 77));
            assert_eq!(bytes[0], 3);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x09);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x4D);
        }

        #[test]
        fn write_single_register() {
            let bytes = encode_request_pdu_to_bytes(&Request::WriteSingleRegister(0x07, 0xABCD));
            assert_eq!(bytes[0], 6);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x07);
            assert_eq!(bytes[3], 0xAB);
            assert_eq!(bytes[4], 0xCD);
        }

        #[test]
        fn write_multiple_registers() {
            let bytes = encode_request_pdu_to_bytes(&Request::WriteMultipleRegisters(
                0x06,
                Cow::Borrowed(&[0xABCD, 0xEF12]),
            ));

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
        fn report_server_id() {
            let bytes = encode_request_pdu_to_bytes(&Request::ReportServerId);
            assert_eq!(bytes[0], 0x11);
        }

        #[test]
        fn masked_write_register() {
            let bytes =
                encode_request_pdu_to_bytes(&Request::MaskWriteRegister(0xABCD, 0xEF12, 0x2345));

            // function code
            assert_eq!(bytes[0], 0x16);

            // address
            assert_eq!(bytes[1], 0xAB);
            assert_eq!(bytes[2], 0xCD);

            // and mask
            assert_eq!(bytes[3], 0xEF);
            assert_eq!(bytes[4], 0x12);

            // or mask
            assert_eq!(bytes[5], 0x23);
            assert_eq!(bytes[6], 0x45);
        }

        #[test]
        fn read_write_multiple_registers() {
            let data = [0xABCD, 0xEF12];
            let bytes = encode_request_pdu_to_bytes(&Request::ReadWriteMultipleRegisters(
                0x05,
                51,
                0x03,
                Cow::Borrowed(&data),
            ));

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
            let bytes = encode_request_pdu_to_bytes(&Request::Custom(
                0x55,
                Cow::Borrowed(&[0xCC, 0x88, 0xAA, 0xFF]),
            ));
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
                Request::WriteMultipleCoils(0x3311, Cow::Borrowed(&[true, false, true, true]))
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
                Request::WriteMultipleRegisters(0x06, Cow::Borrowed(&[0xABCD, 0xEF12]))
            );
        }

        #[test]
        fn report_server_id() {
            let bytes = Bytes::from(vec![0x11]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::ReportServerId);
        }

        #[test]
        fn masked_write_register() {
            let bytes = Bytes::from(vec![0x16, 0xAB, 0xCD, 0xEF, 0x12, 0x23, 0x45]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::MaskWriteRegister(0xABCD, 0xEF12, 0x2345));
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
            let data = [0xABCD, 0xEF12];
            assert_eq!(
                req,
                Request::ReadWriteMultipleRegisters(0x05, 51, 0x03, Cow::Borrowed(&data))
            );
        }

        #[test]
        fn read_device_identification() {
            let bytes = Bytes::from(vec![0x2B, 0x0E, 0x01, 0x01]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::ReadDeviceIdentification(ReadCode::Basic, 1));
        }

        #[test]
        fn custom() {
            let bytes = Bytes::from(vec![0x55, 0xCC, 0x88, 0xAA, 0xFF]);
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(
                req,
                Request::Custom(0x55, Cow::Borrowed(&[0xCC, 0x88, 0xAA, 0xFF]))
            );
        }
    }

    mod serialize_responses {

        use super::*;

        #[test]
        fn read_coils() {
            let bytes = encode_response_pdu_to_bytes(&Response::ReadCoils(vec![
                true, false, false, true, false,
            ]));
            assert_eq!(bytes[0], 1);
            assert_eq!(bytes[1], 1);
            assert_eq!(bytes[2], 0b_0000_1001);
        }

        #[test]
        fn read_discrete_inputs() {
            let bytes = encode_response_pdu_to_bytes(&Response::ReadDiscreteInputs(vec![
                true, false, true, true,
            ]));
            assert_eq!(bytes[0], 2);
            assert_eq!(bytes[1], 1);
            assert_eq!(bytes[2], 0b_0000_1101);
        }

        #[test]
        fn write_single_coil() {
            let bytes = encode_response_pdu_to_bytes(&Response::WriteSingleCoil(0x33, true));
            assert_eq!(bytes[0], 5);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x33);
            assert_eq!(bytes[3], 0xFF);
            assert_eq!(bytes[4], 0x00);
        }

        #[test]
        fn write_multiple_coils() {
            let bytes = encode_response_pdu_to_bytes(&Response::WriteMultipleCoils(0x3311, 5));
            assert_eq!(bytes[0], 0x0F);
            assert_eq!(bytes[1], 0x33);
            assert_eq!(bytes[2], 0x11);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x05);
        }

        #[test]
        fn read_input_registers() {
            let bytes = encode_response_pdu_to_bytes(&Response::ReadInputRegisters(vec![
                0xAA00, 0xCCBB, 0xEEDD,
            ]));
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
            let bytes =
                encode_response_pdu_to_bytes(&Response::ReadHoldingRegisters(vec![0xAA00, 0x1111]));
            assert_eq!(bytes[0], 3);
            assert_eq!(bytes[1], 0x04);
            assert_eq!(bytes[2], 0xAA);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x11);
            assert_eq!(bytes[5], 0x11);
        }

        #[test]
        fn read_device_identification() {
            let bytes = encode_response_pdu_to_bytes(&&Response::ReadDeviceIdentification(
                ReadCode::Basic,
                ConformityLevel::RegularIdentificationStreamOnly,
                false,
                0,
                vec![
                    DeviceIdObject {
                        id: 1,
                        value: Bytes::from("ProductCode"),
                    },
                    DeviceIdObject {
                        id: 2,
                        value: Bytes::from("2.1.3"),
                    },
                ],
            ));
            assert_eq!(bytes[0], 0x2B);
            assert_eq!(bytes[1], 0x0E);
            assert_eq!(bytes[2], 0x01);
            assert_eq!(bytes[3], 0x02);
            assert_eq!(bytes[4], 0x00);
            assert_eq!(bytes[5], 0x00);
            assert_eq!(bytes[6], 0x02);
            assert_eq!(bytes[7], 0x01);
            assert_eq!(bytes[8], 11);
            assert_eq!(std::str::from_utf8(&bytes[9..20]).unwrap(), "ProductCode");
            assert_eq!(bytes[20], 0x02);
            assert_eq!(bytes[21], 5);
            assert_eq!(std::str::from_utf8(&bytes[22..27]).unwrap(), "2.1.3");
        }

        #[test]
        fn write_single_register() {
            let bytes = encode_response_pdu_to_bytes(&Response::WriteSingleRegister(0x07, 0xABCD));
            assert_eq!(bytes[0], 6);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x07);
            assert_eq!(bytes[3], 0xAB);
            assert_eq!(bytes[4], 0xCD);
        }

        #[test]
        fn write_multiple_registers() {
            let bytes = encode_response_pdu_to_bytes(&Response::WriteMultipleRegisters(0x06, 2));
            assert_eq!(bytes[0], 0x10);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x06);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x02);
        }

        #[test]
        fn report_server_id() {
            let bytes = encode_response_pdu_to_bytes(&Response::ReportServerId(
                0x42,
                true,
                vec![0x10, 0x20],
            ));
            assert_eq!(bytes[0], 0x11);
            assert_eq!(bytes[1], 0x04);
            assert_eq!(bytes[2], 0x42);
            assert_eq!(bytes[3], 0xFF);
            assert_eq!(bytes[4], 0x10);
            assert_eq!(bytes[5], 0x20);
        }

        #[test]
        fn masked_write_register() {
            let bytes =
                encode_response_pdu_to_bytes(&Response::MaskWriteRegister(0x06, 0x8001, 0x4002));
            assert_eq!(bytes[0], 0x16);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x06);
            assert_eq!(bytes[3], 0x80);
            assert_eq!(bytes[4], 0x01);
            assert_eq!(bytes[5], 0x40);
            assert_eq!(bytes[6], 0x02);
        }

        #[test]
        fn read_write_multiple_registers() {
            let bytes =
                encode_response_pdu_to_bytes(&Response::ReadWriteMultipleRegisters(vec![0x1234]));
            assert_eq!(bytes[0], 0x17);
            assert_eq!(bytes[1], 0x02);
            assert_eq!(bytes[2], 0x12);
            assert_eq!(bytes[3], 0x34);
        }

        #[test]
        fn custom() {
            let bytes = encode_response_pdu_to_bytes(&Response::Custom(
                0x55,
                Bytes::from_static(&[0xCC, 0x88, 0xAA, 0xFF]),
            ));
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
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(
                response,
                Response::ReadCoils(vec![true, false, false, true, false, false, false, false])
            );
        }

        #[test]
        fn read_coils_max_quantity() {
            let quantity = 2000;
            let byte_count = quantity / 8;
            let mut raw: Vec<u8> = vec![1, u8_len(byte_count)];
            let mut values: Vec<u8> = (0..byte_count).map(|_| 0b_1111_1111).collect();
            raw.append(&mut values);
            let bytes = Bytes::from(raw);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(response, Response::ReadCoils(vec![true; quantity]));
        }

        #[test]
        fn read_discrete_inputs() {
            let bytes = Bytes::from(vec![2, 1, 0b_0000_1001]);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(
                response,
                Response::ReadDiscreteInputs(vec![
                    true, false, false, true, false, false, false, false,
                ],)
            );
        }

        #[test]
        fn read_discrete_inputs_max_quantity() {
            let quantity = 2000;
            let byte_count = quantity / 8;
            let mut raw: Vec<u8> = vec![2, u8_len(byte_count)];
            let mut values: Vec<u8> = (0..byte_count).map(|_| 0b_1111_1111).collect();
            raw.append(&mut values);
            let bytes = Bytes::from(raw);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(response, Response::ReadDiscreteInputs(vec![true; quantity]));
        }

        #[test]
        fn read_device_identification() {
            let bytes = Bytes::from(vec![
                0x2B, 0x0E, 0x01, 0x02, 0x00, 0x00, 0x02, 0x01, 11, b'P', b'r', b'o', b'd', b'u',
                b'c', b't', b'C', b'o', b'd', b'e', 0x02, 5, b'2', b'.', b'1', b'.', b'3',
            ]);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(
                response,
                Response::ReadDeviceIdentification(
                    ReadCode::Basic,
                    ConformityLevel::RegularIdentificationStreamOnly,
                    false,
                    0,
                    vec![
                        DeviceIdObject {
                            id: 1,
                            value: Bytes::from("ProductCode"),
                        },
                        DeviceIdObject {
                            id: 2,
                            value: Bytes::from("2.1.3"),
                        },
                    ],
                )
            );
        }

        #[test]
        fn write_single_coil() {
            let bytes = Bytes::from(vec![5, 0x00, 0x33, 0xFF, 0x00]);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(response, Response::WriteSingleCoil(0x33, true));
        }

        #[test]
        fn write_multiple_coils() {
            let bytes = Bytes::from(vec![0x0F, 0x33, 0x11, 0x00, 0x05]);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(response, Response::WriteMultipleCoils(0x3311, 5));
        }

        #[test]
        fn read_input_registers() {
            let bytes = Bytes::from(vec![4, 0x06, 0xAA, 0x00, 0xCC, 0xBB, 0xEE, 0xDD]);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(
                response,
                Response::ReadInputRegisters(vec![0xAA00, 0xCCBB, 0xEEDD])
            );
        }

        #[test]
        fn read_holding_registers() {
            let bytes = Bytes::from(vec![3, 0x04, 0xAA, 0x00, 0x11, 0x11]);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(
                response,
                Response::ReadHoldingRegisters(vec![0xAA00, 0x1111])
            );
        }

        #[test]
        fn write_single_register() {
            let bytes = Bytes::from(vec![6, 0x00, 0x07, 0xAB, 0xCD]);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(response, Response::WriteSingleRegister(0x07, 0xABCD));
        }

        #[test]
        fn write_multiple_registers() {
            let bytes = Bytes::from(vec![0x10, 0x00, 0x06, 0x00, 0x02]);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(response, Response::WriteMultipleRegisters(0x06, 2));
        }

        #[test]
        fn report_server_id() {
            let bytes = Bytes::from(vec![0x11, 0x04, 0x042, 0xFF, 0x10, 0x20]);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(
                response,
                Response::ReportServerId(0x42, true, vec![0x10, 0x20])
            );
        }

        #[test]
        fn masked_write_register() {
            let bytes = Bytes::from(vec![0x16, 0x00, 0x06, 0x80, 0x01, 0x40, 0x02]);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(response, Response::MaskWriteRegister(6, 0x8001, 0x4002));
        }

        #[test]
        fn read_write_multiple_registers() {
            let bytes = Bytes::from(vec![0x17, 0x02, 0x12, 0x34]);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(response, Response::ReadWriteMultipleRegisters(vec![0x1234]));
        }

        #[test]
        fn custom() {
            let bytes = Bytes::from(vec![0x55, 0xCC, 0x88, 0xAA, 0xFF]);
            let response = Response::try_from(bytes).unwrap();
            assert_eq!(
                response,
                Response::Custom(0x55, Bytes::from_static(&[0xCC, 0x88, 0xAA, 0xFF]))
            );
        }
    }
}
