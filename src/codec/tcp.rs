// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::io::{Error, ErrorKind, Result};

use byteorder::{BigEndian, ByteOrder};
use tokio_util::codec::{Decoder, Encoder};

use crate::{
    bytes::{BufMut, Bytes, BytesMut},
    frame::tcp::*,
};

use super::*;

const HEADER_LEN: usize = 7;

const PROTOCOL_ID: u16 = 0x0000; // TCP

#[derive(Debug, PartialEq)]
pub(crate) struct AduDecoder;

#[derive(Debug, PartialEq)]
pub(crate) struct ClientCodec {
    pub(crate) decoder: AduDecoder,
}

impl Default for ClientCodec {
    fn default() -> Self {
        Self {
            decoder: AduDecoder,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct ServerCodec {
    pub(crate) decoder: AduDecoder,
}

impl Default for ServerCodec {
    fn default() -> Self {
        Self {
            decoder: AduDecoder,
        }
    }
}

impl Decoder for AduDecoder {
    type Item = (Header, Bytes);
    type Error = Error;

    #[allow(clippy::assertions_on_constants)]
    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<(Header, Bytes)>> {
        if buf.len() < HEADER_LEN {
            return Ok(None);
        }

        debug_assert!(HEADER_LEN >= 6);
        let len = usize::from(BigEndian::read_u16(&buf[4..6]));
        let pdu_len = if len > 0 {
            // len = bytes of PDU + one byte (unit ID)
            len - 1
        } else {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid data length: {len}"),
            ));
        };
        if buf.len() < HEADER_LEN + pdu_len {
            return Ok(None);
        }

        let header_data = buf.split_to(HEADER_LEN);

        debug_assert!(HEADER_LEN >= 4);
        let protocol_id = BigEndian::read_u16(&header_data[2..4]);
        if protocol_id != PROTOCOL_ID {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Invalid protocol identifier: expected = {PROTOCOL_ID}, actual = {protocol_id}"
                ),
            ));
        }

        debug_assert!(HEADER_LEN >= 2);
        let transaction_id = BigEndian::read_u16(&header_data[0..2]);

        debug_assert!(HEADER_LEN > 6);
        let unit_id = header_data[6];

        let header = Header {
            transaction_id,
            unit_id,
        };

        let pdu_data = buf.split_to(pdu_len).freeze();

        Ok(Some((header, pdu_data)))
    }
}

impl Decoder for ClientCodec {
    type Item = ResponseAdu;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<ResponseAdu>> {
        if let Some((hdr, pdu_data)) = self.decoder.decode(buf)? {
            let pdu = ResponsePdu::try_from(pdu_data)?;
            Ok(Some(ResponseAdu { hdr, pdu }))
        } else {
            Ok(None)
        }
    }
}

impl Decoder for ServerCodec {
    type Item = RequestAdu<'static>;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<RequestAdu<'static>>> {
        if let Some((hdr, pdu_data)) = self.decoder.decode(buf)? {
            let pdu = RequestPdu::try_from(pdu_data)?;
            Ok(Some(RequestAdu {
                hdr,
                pdu,
                disconnect: false,
            }))
        } else {
            Ok(None)
        }
    }
}

impl<'a> Encoder<RequestAdu<'a>> for ClientCodec {
    type Error = Error;

    fn encode(&mut self, adu: RequestAdu<'a>, buf: &mut BytesMut) -> Result<()> {
        if adu.disconnect {
            // The disconnect happens implicitly after letting this request
            // fail by returning an error. This will drop the attached
            // transport, e.g. for terminating an open connection.
            return Err(Error::new(
                ErrorKind::NotConnected,
                "Disconnecting - not an error",
            ));
        }
        let RequestAdu { hdr, pdu, .. } = adu;
        let pdu_data: Bytes = pdu.try_into()?;
        buf.reserve(pdu_data.len() + 7);
        buf.put_u16(hdr.transaction_id);
        buf.put_u16(PROTOCOL_ID);
        buf.put_u16(u16_len(pdu_data.len() + 1));
        buf.put_u8(hdr.unit_id);
        buf.put_slice(&pdu_data);
        Ok(())
    }
}

impl Encoder<ResponseAdu> for ServerCodec {
    type Error = Error;

    fn encode(&mut self, adu: ResponseAdu, buf: &mut BytesMut) -> Result<()> {
        let ResponseAdu { hdr, pdu } = adu;
        let pdu_data: Bytes = pdu.into();
        buf.reserve(pdu_data.len() + 7);
        buf.put_u16(hdr.transaction_id);
        buf.put_u16(PROTOCOL_ID);
        buf.put_u16(u16_len(pdu_data.len() + 1));
        buf.put_u8(hdr.unit_id);
        buf.put_slice(&pdu_data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytes::Bytes;

    mod client {

        use super::*;

        const TRANSACTION_ID: TransactionId = 0x1001;
        const TRANSACTION_ID_HI: u8 = 0x10;
        const TRANSACTION_ID_LO: u8 = 0x01;

        const PROTOCOL_ID_HI: u8 = (PROTOCOL_ID >> 8) as u8;
        const PROTOCOL_ID_LO: u8 = (PROTOCOL_ID & 0xFF) as u8;

        const UNIT_ID: UnitId = 0xFE;

        #[test]
        fn decode_header_fragment() {
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::from(&[0x00, 0x11, 0x00, 0x00, 0x00, 0x00][..]);
            let res = codec.decode(&mut buf).unwrap();
            assert!(res.is_none());
            assert_eq!(buf.len(), 6);
        }

        #[test]
        fn decode_partly_received_message() {
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::from(
                &[
                    TRANSACTION_ID_HI,
                    TRANSACTION_ID_LO,
                    PROTOCOL_ID_HI,
                    PROTOCOL_ID_LO,
                    0x00, // length high HI
                    0x03, // length low LO
                    UNIT_ID,
                    0x02, // function code
                ][..],
            );
            let res = codec.decode(&mut buf).unwrap();
            assert!(res.is_none());
            assert_eq!(buf.len(), 8);
        }

        #[test]
        fn decode_exception_message() {
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::from(
                &[
                    TRANSACTION_ID_HI,
                    TRANSACTION_ID_LO,
                    PROTOCOL_ID_HI,
                    PROTOCOL_ID_LO,
                    0x00, // length high HI
                    0x03, // length low LO
                    UNIT_ID,
                    0x82, // exception = 0x80 + 0x02
                    0x03, //
                    0x00, //
                ][..],
            );

            let ResponseAdu { hdr, pdu } = codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(hdr.transaction_id, TRANSACTION_ID);
            assert_eq!(hdr.unit_id, UNIT_ID);
            if let ResponsePdu(Err(err)) = pdu {
                assert_eq!(format!("{err}"), "Modbus function 2: Illegal data value");
                assert_eq!(buf.len(), 1);
            } else {
                panic!("unexpected response")
            }
        }

        #[test]
        fn decode_with_invalid_protocol_id() {
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::from(
                &[
                    TRANSACTION_ID_HI,
                    TRANSACTION_ID_LO,
                    0x33, // protocol id HI
                    0x12, // protocol id LO
                    0x00, // length HI
                    0x03, // length LO
                    UNIT_ID,
                ][..],
            );
            buf.extend_from_slice(&[0x00, 0x02, 0x66, 0x82, 0x03, 0x00]);
            let err = codec.decode(&mut buf).err().unwrap();
            assert_eq!(err.kind(), ErrorKind::InvalidData);
            assert!(format!("{err}").contains("Invalid protocol identifier"));
        }

        #[test]
        fn encode_read_request() {
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::new();
            let req = Request::ReadInputRegisters(0x23, 5);
            let pdu = req.clone().into();
            let hdr = Header {
                transaction_id: TRANSACTION_ID,
                unit_id: UNIT_ID,
            };
            let adu = RequestAdu {
                hdr,
                pdu,
                disconnect: false,
            };
            codec.encode(adu, &mut buf).unwrap();
            // header
            assert_eq!(buf[0], TRANSACTION_ID_HI);
            assert_eq!(buf[1], TRANSACTION_ID_LO);
            assert_eq!(buf[2], PROTOCOL_ID_HI);
            assert_eq!(buf[3], PROTOCOL_ID_LO);
            assert_eq!(buf[4], 0x0);
            assert_eq!(buf[5], 0x6);
            assert_eq!(buf[6], UNIT_ID);

            drop(buf.split_to(7));
            let pdu: Bytes = req.try_into().unwrap();
            assert_eq!(buf, pdu);
        }

        #[test]
        fn encode_with_limited_buf_capacity() {
            let mut codec = ClientCodec::default();
            let pdu = Request::ReadInputRegisters(0x23, 5).into();
            let hdr = Header {
                transaction_id: TRANSACTION_ID,
                unit_id: UNIT_ID,
            };
            let adu = RequestAdu {
                hdr,
                pdu,
                disconnect: false,
            };
            let mut buf = BytesMut::with_capacity(40);
            #[allow(unsafe_code)]
            unsafe {
                buf.set_len(29);
            }
            assert!(codec.encode(adu, &mut buf).is_ok());
        }
    }
}
