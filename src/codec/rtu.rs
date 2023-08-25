// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::io::{Cursor, Error, ErrorKind, Result};

use byteorder::BigEndian;
use smallvec::SmallVec;
use tokio_util::codec::{Decoder, Encoder};

use crate::{
    bytes::{Buf, BufMut, Bytes, BytesMut},
    frame::rtu::*,
    slave::SlaveId,
};

use super::*;

// [Modbus over Serial Line Specification and Implementation Guide V1.02](http://modbus.org/docs/Modbus_over_serial_line_V1_02.pdf), page 13
// "The maximum size of a Modbus RTU frame is 256 bytes."
const MAX_FRAME_LEN: usize = 256;

type DroppedBytes = SmallVec<[u8; MAX_FRAME_LEN]>;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct FrameDecoder {
    dropped_bytes: SmallVec<[u8; MAX_FRAME_LEN]>,
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self {
            dropped_bytes: DroppedBytes::new(),
        }
    }
}

impl FrameDecoder {
    pub(crate) fn decode(
        &mut self,
        buf: &mut BytesMut,
        pdu_len: usize,
    ) -> Result<Option<(SlaveId, Bytes)>> {
        const CRC_BYTE_COUNT: usize = 2;

        let adu_len = 1 + pdu_len;

        if buf.len() < adu_len + CRC_BYTE_COUNT {
            // Incomplete frame
            return Ok(None);
        }

        let mut adu_buf = buf.split_to(adu_len);
        let crc_buf = buf.split_to(CRC_BYTE_COUNT);

        // Read trailing CRC and verify ADU
        let crc_result = Cursor::new(&crc_buf)
            .read_u16::<BigEndian>()
            .and_then(|crc| check_crc(&adu_buf, crc));

        if let Err(err) = crc_result {
            // CRC is invalid - restore the input buffer
            let rem_buf = buf.split();
            debug_assert!(buf.is_empty());
            buf.unsplit(adu_buf);
            buf.unsplit(crc_buf);
            buf.unsplit(rem_buf);

            return Err(err);
        }

        if !self.dropped_bytes.is_empty() {
            log::warn!(
                "Successfully decoded frame after dropping {} byte(s): {:X?}",
                self.dropped_bytes.len(),
                self.dropped_bytes
            );
            self.dropped_bytes.clear();
        }
        let slave_id = adu_buf.split_to(1)[0];
        let pdu_data = adu_buf.freeze();

        Ok(Some((slave_id, pdu_data)))
    }

    pub(crate) fn recover_on_error(&mut self, buf: &mut BytesMut) {
        // If decoding failed the buffer cannot be empty
        debug_assert!(!buf.is_empty());
        // Skip and record the first byte of the buffer
        {
            let first = buf.first().unwrap();
            log::debug!("Dropped first byte: {:X?}", first);
            if self.dropped_bytes.len() >= MAX_FRAME_LEN {
                log::error!(
                    "Giving up to decode frame after dropping {} byte(s): {:X?}",
                    self.dropped_bytes.len(),
                    self.dropped_bytes
                );
                self.dropped_bytes.clear();
            }
            self.dropped_bytes.push(*first);
        }
        buf.advance(1);
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct RequestDecoder {
    frame_decoder: FrameDecoder,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct ResponseDecoder {
    frame_decoder: FrameDecoder,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct ClientCodec {
    pub(crate) decoder: ResponseDecoder,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct ServerCodec {
    pub(crate) decoder: RequestDecoder,
}

fn get_request_pdu_len(adu_buf: &BytesMut) -> Result<Option<usize>> {
    if let Some(fn_code) = adu_buf.get(1) {
        let len = match fn_code {
            0x01..=0x06 => 5,
            0x07 | 0x0B | 0x0C | 0x11 => 1,
            0x0F | 0x10 => {
                return Ok(adu_buf
                    .get(6)
                    .map(|&byte_count| 6 + usize::from(byte_count)));
            }
            0x16 => 7,
            0x18 => 3,
            0x17 => {
                return Ok(adu_buf
                    .get(10)
                    .map(|&byte_count| 10 + usize::from(byte_count)));
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Invalid function code: 0x{fn_code:0>2X}"),
                ));
            }
        };
        Ok(Some(len))
    } else {
        Ok(None)
    }
}

fn get_response_pdu_len(adu_buf: &BytesMut) -> Result<Option<usize>> {
    if let Some(fn_code) = adu_buf.get(1) {
        #[allow(clippy::match_same_arms)]
        let len = match fn_code {
            0x01..=0x04 | 0x0C | 0x17 => {
                return Ok(adu_buf
                    .get(2)
                    .map(|&byte_count| 2 + usize::from(byte_count)));
            }
            0x05 | 0x06 | 0x0B | 0x0F | 0x10 => 5,
            0x07 => 2,
            0x16 => 7,
            0x18 => {
                if adu_buf.len() > 3 {
                    3 + usize::from(Cursor::new(&adu_buf[2..=3]).read_u16::<BigEndian>()?)
                } else {
                    // incomplete frame
                    return Ok(None);
                }
            }
            0x81..=0xAB => 2,
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Invalid function code: 0x{fn_code:0>2X}"),
                ));
            }
        };
        Ok(Some(len))
    } else {
        Ok(None)
    }
}

fn calc_crc(data: &[u8]) -> u16 {
    let mut crc = 0xFFFF;
    for x in data {
        crc ^= u16::from(*x);
        for _ in 0..8 {
            let crc_odd = (crc & 0x0001) != 0;
            crc >>= 1;
            if crc_odd {
                crc ^= 0xA001;
            }
        }
    }
    crc << 8 | crc >> 8
}

fn check_crc(adu_data: &[u8], expected_crc: u16) -> Result<()> {
    let actual_crc = calc_crc(adu_data);
    if expected_crc != actual_crc {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid CRC: expected = 0x{expected_crc:0>4X}, actual = 0x{actual_crc:0>4X}"),
        ));
    }
    Ok(())
}

impl Decoder for RequestDecoder {
    type Item = (SlaveId, Bytes);
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<(SlaveId, Bytes)>> {
        decode("request", &mut self.frame_decoder, get_request_pdu_len, buf)
    }
}

impl Decoder for ResponseDecoder {
    type Item = (SlaveId, Bytes);
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<(SlaveId, Bytes)>> {
        decode(
            "response",
            &mut self.frame_decoder,
            get_response_pdu_len,
            buf,
        )
    }
}

fn decode<F>(
    pdu_type: &str,
    frame_decoder: &mut FrameDecoder,
    get_pdu_len: F,
    buf: &mut BytesMut,
) -> Result<Option<(SlaveId, Bytes)>>
where
    F: Fn(&BytesMut) -> Result<Option<usize>>,
{
    const MAX_RETRIES: usize = 20;

    for _i in 0..MAX_RETRIES {
        let result = get_pdu_len(buf).and_then(|pdu_len| {
            let Some(pdu_len) = pdu_len else {
                // Incomplete frame
                return Ok(None);
            };

            frame_decoder.decode(buf, pdu_len)
        });

        if let Err(err) = result {
            log::warn!("Failed to decode {pdu_type} frame: {err}");
            frame_decoder.recover_on_error(buf);
            continue;
        }

        return result;
    }

    // Maximum number of retries exceeded.
    log::error!("Giving up to decode frame after {MAX_RETRIES} retries");
    Err(Error::new(ErrorKind::InvalidData, "Too many retries"))
}

impl Decoder for ClientCodec {
    type Item = ResponseAdu;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<ResponseAdu>> {
        let Some((slave_id, pdu_data)) = self.decoder.decode(buf)? else {
            return Ok(None);
        };

        let hdr = Header { slave_id };

        // Decoding of the PDU is unlikely to fail due
        // to transmission errors, because the frame's bytes
        // have already been verified with the CRC.
        ResponsePdu::try_from(pdu_data)
            .map(|pdu| Some(ResponseAdu { hdr, pdu }))
            .map_err(|err| {
                // Unrecoverable error
                log::error!("Failed to decode response PDU: {}", err);
                err
            })
    }
}

impl Decoder for ServerCodec {
    type Item = RequestAdu<'static>;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<RequestAdu<'static>>> {
        let Some((slave_id, pdu_data)) = self.decoder.decode(buf)? else {
            return Ok(None);
        };

        let hdr = Header { slave_id };

        // Decoding of the PDU is unlikely to fail due
        // to transmission errors, because the frame's bytes
        // have already been verified with the CRC.
        RequestPdu::try_from(pdu_data)
            .map(|pdu| {
                Some(RequestAdu {
                    hdr,
                    pdu,
                    disconnect: false,
                })
            })
            .map_err(|err| {
                // Unrecoverable error
                log::error!("Failed to decode request PDU: {}", err);
                err
            })
    }
}

impl<'a> Encoder<RequestAdu<'a>> for ClientCodec {
    type Error = Error;

    fn encode(&mut self, adu: RequestAdu<'a>, buf: &mut BytesMut) -> Result<()> {
        if adu.disconnect {
            // The disconnect happens implicitly after letting this request
            // fail by returning an error. This will drop the attached
            // transport, e.g. for closing a stale, exclusive connection
            // to a serial port before trying to reconnect.
            return Err(Error::new(
                ErrorKind::NotConnected,
                "Disconnecting - not an error",
            ));
        }
        let RequestAdu { hdr, pdu, .. } = adu;
        let pdu_data: Bytes = pdu.try_into()?;
        buf.reserve(pdu_data.len() + 3);
        buf.put_u8(hdr.slave_id);
        buf.put_slice(&pdu_data);
        let crc = calc_crc(buf);
        buf.put_u16(crc);
        Ok(())
    }
}

impl Encoder<ResponseAdu> for ServerCodec {
    type Error = Error;

    fn encode(&mut self, adu: ResponseAdu, buf: &mut BytesMut) -> Result<()> {
        let ResponseAdu { hdr, pdu } = adu;
        let pdu_data: Bytes = pdu.into();
        buf.reserve(pdu_data.len() + 3);
        buf.put_u8(hdr.slave_id);
        buf.put_slice(&pdu_data);
        let crc = calc_crc(buf);
        buf.put_u16(crc);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytes::Bytes;

    #[test]
    fn test_calc_crc() {
        let msg = [0x01, 0x03, 0x08, 0x2B, 0x00, 0x02];
        assert_eq!(calc_crc(&msg), 0xB663);

        let msg = [0x01, 0x03, 0x04, 0x00, 0x20, 0x00, 0x00];
        assert_eq!(calc_crc(&msg), 0xFBF9);
    }

    #[test]
    fn test_get_request_pdu_len() {
        let mut buf = BytesMut::new();

        buf.extend_from_slice(&[0x66, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(get_request_pdu_len(&buf).is_err());

        buf[1] = 0x01;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(5));

        buf[1] = 0x02;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(5));

        buf[1] = 0x03;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(5));

        buf[1] = 0x04;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(5));

        buf[1] = 0x05;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(5));

        buf[1] = 0x06;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(5));

        buf[1] = 0x07;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(1));

        // TODO: 0x08

        buf[1] = 0x0B;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(1));

        buf[1] = 0x0C;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(1));

        buf[1] = 0x0F;
        buf[6] = 99;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(105));

        buf[1] = 0x10;
        buf[6] = 99;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(105));

        buf[1] = 0x11;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(1));

        // TODO: 0x14

        // TODO: 0x15

        buf[1] = 0x16;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(7));

        buf[1] = 0x17;
        buf[10] = 99; // write byte count
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(109));

        buf[1] = 0x18;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(3));

        // TODO: 0x2B
    }

    #[test]
    fn test_get_response_pdu_len() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x66, 0x01, 99]);
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(101));

        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x66, 0x00, 99, 0x00]);
        assert!(get_response_pdu_len(&buf).is_err());

        buf[1] = 0x01;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(101));

        buf[1] = 0x02;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(101));

        buf[1] = 0x03;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(101));

        buf[1] = 0x04;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(101));

        buf[1] = 0x05;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(5));

        buf[1] = 0x06;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(5));

        buf[1] = 0x07;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(2));

        // TODO: 0x08

        buf[1] = 0x0B;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(5));

        buf[1] = 0x0C;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(101));

        buf[1] = 0x0F;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(5));

        buf[1] = 0x10;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(5));

        // TODO: 0x11

        // TODO: 0x14

        // TODO: 0x15

        buf[1] = 0x16;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(7));

        buf[1] = 0x17;
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(101));

        buf[1] = 0x18;
        buf[2] = 0x01; // byte count Hi
        buf[3] = 0x00; // byte count Lo
        assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(259));

        // TODO: 0x2B

        for i in 0x81..0xAB {
            buf[1] = i;
            assert_eq!(get_response_pdu_len(&buf).unwrap(), Some(2));
        }
    }

    mod client {

        use super::*;

        #[test]
        fn decode_partly_received_client_message() {
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::from(
                &[
                    0x12, // slave address
                    0x02, // function code
                    0x03, // byte count
                    0x00, // data
                    0x00, // data
                    0x00, // data
                    0x00, // CRC first byte
                          // missing crc second byte
                ][..],
            );
            let res = codec.decode(&mut buf).unwrap();
            assert!(res.is_none());
            assert_eq!(buf.len(), 7);
        }

        #[test]
        fn decode_empty_client_message() {
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::new();
            assert_eq!(0, buf.len());

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(0, buf.len());
        }

        #[test]
        fn decode_single_byte_client_message() {
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::from(&[0x00][..]);
            assert_eq!(1, buf.len());

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(1, buf.len());
        }

        #[test]
        fn decode_empty_server_message() {
            let mut codec = ServerCodec::default();
            let mut buf = BytesMut::new();
            assert_eq!(0, buf.len());

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(0, buf.len());
        }

        #[test]
        fn decode_single_byte_server_message() {
            let mut codec = ServerCodec::default();
            let mut buf = BytesMut::from(&[0x00][..]);
            assert_eq!(1, buf.len());

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(1, buf.len());
        }

        #[test]
        fn decode_partly_received_server_message_0x16() {
            let mut codec = ServerCodec::default();
            let mut buf = BytesMut::from(
                &[
                    0x12, // slave address
                    0x16, // function code
                ][..],
            );
            assert_eq!(buf.len(), 2);

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(buf.len(), 2);
        }

        #[test]
        fn decode_partly_received_server_message_0x0f() {
            let mut codec = ServerCodec::default();
            let mut buf = BytesMut::from(
                &[
                    0x12, // slave address
                    0x0F, // function code
                ][..],
            );
            assert_eq!(buf.len(), 2);

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(buf.len(), 2);
        }

        #[test]
        fn decode_partly_received_server_message_0x10() {
            let mut codec = ServerCodec::default();
            let mut buf = BytesMut::from(
                &[
                    0x12, // slave address
                    0x10, // function code
                ][..],
            );
            assert_eq!(buf.len(), 2);

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(buf.len(), 2);
        }

        #[test]
        fn decode_rtu_message() {
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::from(
                &[
                    0x01, // slave address
                    0x03, // function code
                    0x04, // byte count
                    0x89, //
                    0x02, //
                    0x42, //
                    0xC7, //
                    0x00, // crc
                    0x9D, // crc
                    0x00,
                ][..],
            );
            let ResponseAdu { hdr, pdu } = codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(buf.len(), 1);
            assert_eq!(hdr.slave_id, 0x01);
            if let Ok(Response::ReadHoldingRegisters(data)) = pdu.into() {
                assert_eq!(data.len(), 2);
                assert_eq!(data, vec![0x8902, 0x42C7]);
            } else {
                panic!("unexpected response")
            }
        }

        #[test]
        fn decode_rtu_response_drop_invalid_bytes() {
            env_logger::init();
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::from(
                &[
                    0x42, // dropped byte
                    0x43, // dropped byte
                    0x01, // slave address
                    0x03, // function code
                    0x04, // byte count
                    0x89, //
                    0x02, //
                    0x42, //
                    0xC7, //
                    0x00, // crc
                    0x9D, // crc
                    0x00,
                ][..],
            );
            let ResponseAdu { hdr, pdu } = codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(buf.len(), 1);
            assert_eq!(hdr.slave_id, 0x01);
            if let Ok(Response::ReadHoldingRegisters(data)) = pdu.into() {
                assert_eq!(data.len(), 2);
                assert_eq!(data, vec![0x8902, 0x42C7]);
            } else {
                panic!("unexpected response")
            }
        }

        #[test]
        fn decode_exception_message() {
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::from(
                &[
                    0x66, //
                    0x82, // exception = 0x80 + 0x02
                    0x03, //
                    0xB1, // crc
                    0x7E, // crc
                ][..],
            );

            let ResponseAdu { pdu, .. } = codec.decode(&mut buf).unwrap().unwrap();
            if let ResponsePdu(Err(err)) = pdu {
                assert_eq!(format!("{err}"), "Modbus function 2: Illegal data value");
                assert_eq!(buf.len(), 0);
            } else {
                panic!("unexpected response")
            }
        }

        #[test]
        fn encode_read_request() {
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::new();
            let req = Request::ReadHoldingRegisters(0x082b, 2);
            let pdu = req.into();
            let slave_id = 0x01;
            let hdr = Header { slave_id };
            let adu = RequestAdu {
                hdr,
                pdu,
                disconnect: false,
            };
            codec.encode(adu, &mut buf).unwrap();

            assert_eq!(
                buf,
                Bytes::from_static(&[0x01, 0x03, 0x08, 0x2B, 0x00, 0x02, 0xB6, 0x63])
            );
        }

        #[test]
        fn encode_with_limited_buf_capacity() {
            let mut codec = ClientCodec::default();
            let req = Request::ReadHoldingRegisters(0x082b, 2);
            let pdu = req.into();
            let slave_id = 0x01;
            let hdr = Header { slave_id };
            let adu = RequestAdu {
                hdr,
                pdu,
                disconnect: false,
            };
            let mut buf = BytesMut::with_capacity(40);
            #[allow(unsafe_code)]
            unsafe {
                buf.set_len(33);
            }
            assert!(codec.encode(adu, &mut buf).is_ok());
        }
    }
}
