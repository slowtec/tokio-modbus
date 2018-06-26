use super::common::*;
use byteorder::ByteOrder;
use bytes::{BigEndian, BufMut, Bytes, BytesMut};
use frame::*;
use std::io::{Error, ErrorKind, Result};
use tokio_codec::{Decoder, Encoder};

const HEADER_SIZE: usize = 7;
const PROTOCOL_ID: u16 = 0x0;

#[derive(Debug, PartialEq)]
pub(crate) struct TcpDecoder;

#[derive(Debug, PartialEq)]
pub(crate) struct Codec {
    pub(crate) decoder: TcpDecoder,
    pub(crate) codec_type: CodecType,
}

impl Codec {
    pub fn client() -> Codec {
        Codec {
            decoder: TcpDecoder,
            codec_type: CodecType::Client,
        }
    }
    pub fn server() -> Codec {
        Codec {
            decoder: TcpDecoder,
            codec_type: CodecType::Server,
        }
    }
}

impl Decoder for TcpDecoder {
    type Item = (TcpHeader, Bytes);
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<(TcpHeader, Bytes)>> {
        if buf.len() < HEADER_SIZE {
            return Ok(None);
        }

        // len = bytes of PDU + one byte (unit ID)
        let len = BigEndian::read_u16(&buf[4..6]) as usize;

        if buf.len() < HEADER_SIZE + len - 1 {
            return Ok(None);
        }

        let header_data = buf.split_to(HEADER_SIZE);
        let data = buf.split_to(len - 1).freeze();

        let transaction_id = BigEndian::read_u16(&header_data[0..2]);
        let protocol_id = BigEndian::read_u16(&header_data[2..4]);
        let unit_id = header_data[4];

        if protocol_id != PROTOCOL_ID {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid protocol ID"));
        }

        let header = TcpHeader {
            transaction_id,
            unit_id,
        };
        Ok(Some((header, data)))
    }
}

impl Decoder for Codec {
    type Item = TcpAdu;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<TcpAdu>> {
        if let Some((header, data)) = self.decoder.decode(buf)? {
            let pdu = match self.codec_type {
                CodecType::Client => {
                    let res = if data[0] > 0x80 {
                        Err(ExceptionResponse::try_from(data)?)
                    } else {
                        Ok(Response::try_from(data)?)
                    };
                    Pdu::Result(res)
                }
                CodecType::Server => {
                    if data[0] > 0x80 {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "A request must not a exception response",
                        ));
                    }
                    let req = Request::try_from(data)?;
                    Pdu::Request(req)
                }
            };
            Ok(Some(TcpAdu { header, pdu }))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for Codec {
    type Item = TcpAdu;
    type Error = Error;

    fn encode(&mut self, adu: TcpAdu, buf: &mut BytesMut) -> Result<()> {
        let TcpAdu { header, pdu } = adu;
        let pdu: Bytes = pdu.into();
        buf.reserve(pdu.len() + 7);
        buf.put_u16_be(header.transaction_id);
        buf.put_u16_be(PROTOCOL_ID);
        buf.put_u16_be((pdu.len() + 1) as u16);
        buf.put_u8(header.unit_id);
        buf.put_slice(&*pdu);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    mod client {

        use super::*;

        #[test]
        fn decode_header_fragment() {
            let mut codec = Codec::client();
            let mut buf = BytesMut::from(vec![0x00, 0x11, 0x00, 0x00, 0x00, 0x00]);
            let res = codec.decode(&mut buf).unwrap();
            assert!(res.is_none());
            assert_eq!(buf.len(), 6);
        }

        #[test]
        fn decode_partly_received_message() {
            let mut codec = Codec::client();
            let mut buf = BytesMut::from(vec![
                0x00, // transaction id HI
                0x11, // transaction id LO
                0x00, // prototcol id HI
                0x00, // prototcol id LO
                0x00, // length high HI
                0x03, // length low LO
                0x66, // unit id
                0x02,
            ]);
            let res = codec.decode(&mut buf).unwrap();
            assert!(res.is_none());
            assert_eq!(buf.len(), 8);
        }

        #[test]
        fn decode_exception_message() {
            let mut codec = Codec::client();
            let mut buf = BytesMut::from(vec![
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x03,
                0x66,
                0x82, // exception = 0x80 + 0x02
                0x03,
                0x00,
            ]);

            let TcpAdu { header, pdu } = codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(header.transaction_id, 0);
            match pdu {
                Pdu::Result(res) => {
                    let err = res.err().unwrap();
                    assert_eq!(format!("{}", err), "Modbus function 2: Illegal data value");
                }
                _ => panic!("wrong pdu type"),
            }
            assert_eq!(buf.len(), 1);
        }

        #[test]
        fn decode_with_invalid_protocol_id() {
            let mut codec = Codec::client();
            let mut buf = BytesMut::from(vec![
                0x00,
                0x00,
                0x33, // protocol id HI
                0x12, // protocol id LO
                0x00, // length HI
                0x03, // length LO
                0x66, // unit id
            ]);
            buf.extend_from_slice(&[0x00, 0x02, 0x66, 0x82, 0x03, 0x00]);
            let err = codec.decode(&mut buf).err().unwrap();
            assert_eq!(err.kind(), ErrorKind::InvalidData);
            assert_eq!(format!("{}", err), "Invalid protocol ID");
        }

        #[test]
        fn encode_read_request() {
            let mut codec = Codec::client();
            let mut buf = BytesMut::new();
            let req = Request::ReadInputRegisters(0x23, 5);
            let pdu = Pdu::Request(req.clone());
            let header = TcpHeader {
                transaction_id: 0,
                unit_id: 0,
            };
            let adu = TcpAdu { header, pdu };
            codec.encode(adu.clone(), &mut buf).unwrap();
            // header
            assert_eq!(buf[0], 0x0);
            assert_eq!(buf[1], 0x0);
            assert_eq!(buf[2], 0x0);
            assert_eq!(buf[3], 0x0);
            assert_eq!(buf[4], 0x0);
            assert_eq!(buf[5], 0x6);
            assert_eq!(buf[6], 0x0);

            buf.split_to(7);
            let pdu: Bytes = req.into();
            assert_eq!(buf, pdu);
        }

        #[test]
        fn encode_transaction_id() {
            let mut codec = Codec::client();
            let pdu = Pdu::Request(Request::ReadInputRegisters(0x00, 1));
            let header = TcpHeader {
                transaction_id: 0xab,
                unit_id: 0,
            };
            let adu = TcpAdu { header, pdu };

            let mut buf = BytesMut::new();
            codec.encode(adu.clone(), &mut buf).unwrap();
            assert_eq!(buf[1], 0xab);
        }

        #[test]
        fn encode_with_limited_buf_capacity() {
            let mut codec = Codec::client();
            let req = Request::ReadInputRegisters(0x23, 5);
            let pdu = Pdu::Request(req);
            let header = TcpHeader {
                transaction_id: 0,
                unit_id: 0,
            };
            let adu = TcpAdu { header, pdu };
            let mut buf = BytesMut::with_capacity(40);
            unsafe {
                buf.set_len(29);
            }
            assert!(codec.encode(adu.clone(), &mut buf).is_ok());
        }
    }
}
