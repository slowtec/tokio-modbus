use super::*;

use crate::frame::rtu::*;

use bytes::{BigEndian, BufMut, Bytes, BytesMut};
use std::io::{Cursor, Error, ErrorKind, Result};
use tokio_codec::{Decoder, Encoder};

#[derive(Debug, PartialEq)]
pub(crate) struct RequestDecoder;

#[derive(Debug, PartialEq)]
pub(crate) struct ResponseDecoder;

#[derive(Debug, PartialEq)]
pub(crate) struct ClientCodec {
    pub(crate) decoder: ResponseDecoder,
}

impl Default for ClientCodec {
    fn default() -> Self {
        Self {
            decoder: ResponseDecoder,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct ServerCodec {
    pub(crate) decoder: RequestDecoder,
}

impl Default for ServerCodec {
    fn default() -> Self {
        Self {
            decoder: RequestDecoder,
        }
    }
}

fn get_request_pdu_len(adu_buf: &BytesMut) -> Result<Option<usize>> {
    if adu_buf.len() < 2 {
        return Ok(None);
    }
    let fn_code = adu_buf[1];
    let len = match fn_code {
        0x01...0x06 => Some(5),
        0x07 | 0x0B | 0x0C | 0x11 => Some(1),
        0x0F | 0x10 => {
            if adu_buf.len() > 4 {
                Some(6 + adu_buf[4] as usize)
            } else {
                // incomplete frame
                None
            }
        }
        0x16 => Some(7),
        0x18 => Some(3),
        0x17 => {
            if adu_buf.len() > 10 {
                Some(10 + adu_buf[10] as usize)
            } else {
                // incomplete frame
                None
            }
        }
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid function code: 0x{:0>2X}", fn_code),
            ))
        }
    };
    Ok(len)
}

fn get_response_pdu_len(adu_buf: &BytesMut) -> Result<Option<usize>> {
    if adu_buf.len() < 2 {
        return Ok(None);
    }
    let fn_code = adu_buf[1];
    let len = match fn_code {
        0x01...0x04 | 0x0C | 0x17 => {
            if adu_buf.len() > 2 {
                Some(2 + adu_buf[2] as usize)
            } else {
                // incomplete frame
                None
            }
        }
        0x05 | 0x06 | 0x0B | 0x0F | 0x10 => Some(5),
        0x07 => Some(2),
        0x16 => Some(7),
        0x18 => {
            if adu_buf.len() > 3 {
                Some(3 + Cursor::new(&adu_buf[2..=3]).read_u16::<BigEndian>()? as usize)
            } else {
                // incomplete frame
                None
            }
        }
        0x81...0xAB => Some(2),
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid function code: 0x{:0>2X}", fn_code),
            ))
        }
    };
    Ok(len)
}

fn calc_crc(data: &[u8]) -> u16 {
    let mut crc = 0xFFFF;
    for x in data {
        crc ^= u16::from(*x);
        for _ in 0..8 {
            if (crc & 0x0001) != 0 {
                crc >>= 1;
                crc ^= 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    (crc << 8 | crc >> 8)
}

fn check_crc(adu_data: &[u8], expected_crc: u16) -> Result<()> {
    let actual_crc = calc_crc(&adu_data);
    if expected_crc != actual_crc {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Invalid CRC: expected = 0x{:0>4X}, actual = 0x{:0>4X}",
                expected_crc, actual_crc
            ),
        ));
    }
    Ok(())
}

impl Decoder for RequestDecoder {
    type Item = (SlaveAddress, Bytes);
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<(SlaveAddress, Bytes)>> {
        if let Some(pdu_len) = get_request_pdu_len(buf)? {
            let adu_len = 1 + pdu_len;
            if buf.len() >= adu_len + 2 {
                let mut adu_data = buf.split_to(adu_len);
                // Read trailing CRC and verify ADU
                let crc = Cursor::new(&buf.split_to(2)).read_u16::<BigEndian>()?;
                check_crc(&adu_data, crc)?;
                let slave_addr = adu_data.split_to(1)[0];
                let pdu_data = adu_data.freeze();
                return Ok(Some((slave_addr, pdu_data)));
            }
        }
        // incomplete frame
        Ok(None)
    }
}

impl Decoder for ResponseDecoder {
    type Item = (SlaveAddress, Bytes);
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<(SlaveAddress, Bytes)>> {
        if let Some(pdu_len) = get_response_pdu_len(buf)? {
            let adu_len = 1 + pdu_len;
            if buf.len() >= adu_len + 2 {
                let mut adu_data = buf.split_to(adu_len);
                // Read trailing CRC and verify ADU
                let crc = Cursor::new(&buf.split_to(2)).read_u16::<BigEndian>()?;
                check_crc(&adu_data, crc)?;
                let slave_addr = adu_data.split_to(1)[0];
                let pdu_data = adu_data.freeze();
                return Ok(Some((slave_addr, pdu_data)));
            }
        }
        Ok(None)
    }
}

impl Decoder for ClientCodec {
    type Item = ResponseAdu;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<ResponseAdu>> {
        if let Some((slave_addr, pdu_data)) = self.decoder.decode(buf)? {
            let hdr = Header { slave_addr };
            let pdu = ResponsePdu::try_from(pdu_data)?;
            Ok(Some(ResponseAdu { hdr, pdu }))
        } else {
            Ok(None)
        }
    }
}

impl Decoder for ServerCodec {
    type Item = RequestAdu;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<RequestAdu>> {
        if let Some((slave_addr, pdu_data)) = self.decoder.decode(buf)? {
            let hdr = Header { slave_addr };
            let pdu = RequestPdu::try_from(pdu_data)?;
            Ok(Some(RequestAdu { hdr, pdu }))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for ClientCodec {
    type Item = RequestAdu;
    type Error = Error;

    fn encode(&mut self, adu: RequestAdu, buf: &mut BytesMut) -> Result<()> {
        let RequestAdu { hdr, pdu } = adu;
        let pdu_data: Bytes = pdu.into();
        buf.reserve(pdu_data.len() + 3);
        buf.put_u8(hdr.slave_addr);
        buf.put_slice(&*pdu_data);
        let crc = calc_crc(buf);
        buf.put_u16_be(crc);
        Ok(())
    }
}

impl Encoder for ServerCodec {
    type Item = ResponseAdu;
    type Error = Error;

    fn encode(&mut self, adu: ResponseAdu, buf: &mut BytesMut) -> Result<()> {
        let ResponseAdu { hdr, pdu } = adu;
        let pdu_data: Bytes = pdu.into();
        buf.reserve(pdu_data.len() + 3);
        buf.put_u8(hdr.slave_addr);
        buf.put_slice(&*pdu_data);
        let crc = calc_crc(buf);
        buf.put_u16_be(crc);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_calc_crc() {
        let msg = vec![0x01, 0x03, 0x08, 0x2B, 0x00, 0x02];
        assert_eq!(calc_crc(&msg), 0xB663);

        let msg = vec![0x01, 0x03, 0x04, 0x00, 0x20, 0x00, 0x00];
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
        buf[4] = 99;
        assert_eq!(get_request_pdu_len(&buf).unwrap(), Some(105));

        buf[1] = 0x10;
        buf[4] = 99;
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
            let mut buf = BytesMut::from(vec![
                0x12, // slave address
                0x02, // function code
                0x03, // byte count
                0x00, // data
                0x00, // data
                0x00, // data
                0x00, // CRC first byte
                      // missing crc second byte
            ]);
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
            let mut buf = BytesMut::from(vec![0x00]);
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
            let mut buf = BytesMut::from(vec![0x00]);
            assert_eq!(1, buf.len());

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(1, buf.len());
        }

        #[test]
        fn decode_partly_received_server_message_0x16() {
            let mut codec = ServerCodec::default();
            let mut buf = BytesMut::from(vec![
                0x12, // slave address
                0x16, // function code
            ]);
            assert_eq!(buf.len(), 2);

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(buf.len(), 2);
        }

        #[test]
        fn decode_partly_received_server_message_0x0f() {
            let mut codec = ServerCodec::default();
            let mut buf = BytesMut::from(vec![
                0x12, // slave address
                0x0F, // function code
            ]);
            assert_eq!(buf.len(), 2);

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(buf.len(), 2);
        }

        #[test]
        fn decode_partly_received_server_message_0x10() {
            let mut codec = ServerCodec::default();
            let mut buf = BytesMut::from(vec![
                0x12, // slave address
                0x10, // function code
            ]);
            assert_eq!(buf.len(), 2);

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(buf.len(), 2);
        }

        #[test]
        fn decode_rtu_message() {
            let mut codec = ClientCodec::default();
            let mut buf = BytesMut::from(vec![
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
            ]);
            let ResponseAdu { hdr, pdu } = codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(buf.len(), 1);
            assert_eq!(hdr.slave_addr, 0x01);
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
            let mut buf = BytesMut::from(vec![
                0x66, //
                0x82, // exception = 0x80 + 0x02
                0x03, //
                0xB1, // crc
                0x7E, // crc
            ]);

            let ResponseAdu { pdu, .. } = codec.decode(&mut buf).unwrap().unwrap();
            if let ResponsePdu(Err(err)) = pdu {
                assert_eq!(format!("{}", err), "Modbus function 2: Illegal data value");
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
            let pdu = req.clone().into();
            let slave_addr = 0x01;
            let hdr = Header { slave_addr };
            let adu = RequestAdu { hdr, pdu };
            codec.encode(adu.clone(), &mut buf).unwrap();

            assert_eq!(
                buf,
                Bytes::from_static(&[0x01, 0x03, 0x08, 0x2B, 0x00, 0x02, 0xB6, 0x63])
            );
        }

        #[test]
        fn encode_with_limited_buf_capacity() {
            let mut codec = ClientCodec::default();
            let req = Request::ReadHoldingRegisters(0x082b, 2);
            let pdu = req.clone().into();
            let slave_addr = 0x01;
            let hdr = Header { slave_addr };
            let adu = RequestAdu { hdr, pdu };
            let mut buf = BytesMut::with_capacity(40);
            unsafe {
                buf.set_len(33);
            }
            assert!(codec.encode(adu, &mut buf).is_ok());
        }
    }
}
