use frame::*;
use std::io::{Error, ErrorKind, Result};
use tokio_io::codec::{Decoder, Encoder};
use bytes::{BigEndian, BufMut, Bytes, BytesMut};
use byteorder::ByteOrder;
use super::common::*;

const HEADER_SIZE: usize = 7;
const PROTOCOL_ID: u16 = 0x0;

pub struct ClientCodec {
    transaction_id: u16,
    unit_id: u8,
}

impl ClientCodec {
    pub fn new() -> ClientCodec {
        ClientCodec {
            transaction_id: 0,
            unit_id: 0,
        }
    }
}

impl Decoder for ClientCodec {
    type Item = ModbusResult;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<ModbusResult>> {
        if buf.len() < HEADER_SIZE {
            return Ok(None);
        }

        // len = bytes of PDU + one byte (unit ID)
        let len = BigEndian::read_u16(&buf[4..6]) as usize;

        if buf.len() < HEADER_SIZE + len - 1 {
            return Ok(None);
        }

        let header_data = buf.split_to(HEADER_SIZE);
        let data = buf.split_to(len-1).freeze();

        let transaction_id = BigEndian::read_u16(&header_data[0..2]);
        let protocol_id = BigEndian::read_u16(&header_data[2..4]);
        let unit_id = header_data[4];

        if transaction_id.wrapping_add(1) != self.transaction_id {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid transaction ID"));
        }

        if protocol_id != PROTOCOL_ID {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid protocol ID"));
        }

        let res = if data[0] > 0x80 {
            Err(ExceptionResponse::try_from(data)?)
        } else {
            Ok(Response::try_from(data)?)
        };

        Ok(Some(res))
    }
}

impl Encoder for ClientCodec {
    type Item = Request;
    type Error = Error;

    fn encode(&mut self, req: Request, buf: &mut BytesMut) -> Result<()> {
        let pdu: Bytes = req.into();
        buf.put_u16::<BigEndian>(self.transaction_id);
        buf.put_u16::<BigEndian>(PROTOCOL_ID);
        buf.put_u16::<BigEndian>((pdu.len() +1) as u16);
        buf.put_u8(self.unit_id);
        buf.extend_from_slice(&*pdu);
        self.transaction_id = self.transaction_id.wrapping_add(1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod client {

        use super::*;

        #[test]
        fn decode_header_fragment() {
            let mut codec = ClientCodec::new();
            let mut buf = BytesMut::from(vec![0x00, 0x11, 0x00, 0x00, 0x00, 0x00]);
            let res = codec.decode(&mut buf).unwrap();
            assert!(res.is_none());
            assert_eq!(buf.len(), 6);
        }

        #[test]
        fn decode_partly_received_message() {
            let mut codec = ClientCodec::new();
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
            let mut codec = ClientCodec::new();
            codec.transaction_id = 1; // incremented on send
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
            let res = codec.decode(&mut buf).unwrap().unwrap();

            assert_eq!(buf.len(), 1);
            assert_eq!(
                res,
                Err(ExceptionResponse {
                    function: 0x02,
                    exception: Exception::IllegalDataValue,
                })
            );
        }

        #[test]
        fn decode_with_invalid_protocol_id() {
            let mut codec = ClientCodec::new();
            codec.transaction_id = 1; // incremented after send
            let mut buf = BytesMut::from(vec![
                                         0x00,
                                         0x00,
                                         0x33, // protocol id HI
                                         0x12, // protocol id LO
                                         0x00, // length HI
                                         0x03, // length LO
                                         0x66  // unit id
            ]);
            buf.extend_from_slice(&[0x00, 0x02, 0x66, 0x82, 0x03, 0x00]);
            let err = codec.decode(&mut buf).err().unwrap();
            assert_eq!(err.kind(), ErrorKind::InvalidData);
            assert_eq!(format!("{}", err), "Invalid protocol ID");
        }

        #[test]
        fn decode_with_invalid_transaction_id() {
            let mut codec = ClientCodec::new();
            assert_eq!(codec.transaction_id, 0);
            let mut buf = BytesMut::from(vec![0x0, 0x7, 0x0, 0x0, 0x0, 0x2, 0x1, 0x2, 0x1]);
            let err = codec.decode(&mut buf).err().unwrap();
            assert_eq!(err.kind(), ErrorKind::InvalidData);
            assert_eq!(format!("{}", err), "Invalid transaction ID");
        }

        #[test]
        fn encode_read_request() {
            let mut codec = ClientCodec::new();
            let mut buf = BytesMut::new();
            let req = Request::ReadInputRegisters(0x23, 5);
            codec.encode(req.clone(), &mut buf).unwrap();
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
            let mut codec = ClientCodec::new();
            let req = Request::ReadInputRegisters(0x00, 1);

            let mut buf = BytesMut::new();
            codec.encode(req.clone(), &mut buf).unwrap();
            assert_eq!(buf[1], 0x0);

            let mut buf = BytesMut::new();
            codec.encode(req.clone(), &mut buf).unwrap();
            assert_eq!(buf[1], 0x1);

            let mut buf = BytesMut::new();
            codec.encode(req.clone(), &mut buf).unwrap();
            assert_eq!(buf[1], 0x2);

            let mut buf = BytesMut::new();
            codec.transaction_id = ::std::u16::MAX;
            codec.encode(req.clone(), &mut buf).unwrap();
            assert_eq!(buf[1], 0xFF);

            let mut buf = BytesMut::new();
            codec.encode(req.clone(), &mut buf).unwrap();
            assert_eq!(buf[1], 0x0);
        }
    }
}
