use super::common::*;
use byteorder::ReadBytesExt;
use bytes::{BigEndian, BufMut, Bytes, BytesMut};
use crate::frame::*;
use std::io::{Cursor, Error, ErrorKind, Result};
use tokio_codec::{Decoder, Encoder};

#[derive(Debug, PartialEq)]
pub(crate) struct Codec {
    pub(crate) decoder: RtuDecoder,
}

#[derive(Debug, PartialEq)]
pub(crate) struct RtuDecoder {
    codec_type: CodecType,
}

const MIN_ADU_LEN: usize = 1 + 1 + 2; // addr + function + crc

impl Codec {
    pub fn client() -> Codec {
        Codec {
            decoder: RtuDecoder {
                codec_type: CodecType::Client,
            },
        }
    }
    pub fn server() -> Codec {
        Codec {
            decoder: RtuDecoder {
                codec_type: CodecType::Server,
            },
        }
    }
}

fn get_request_payload_len(buf: &BytesMut) -> Result<Option<usize>> {
    if buf.len() < 2 {
        // incomplete frame
        return Ok(None);
    }
    let len = match buf[1] {
        0x01...0x06 => Some(4),
        0x07 | 0x0B | 0x0C | 0x11 => Some(0),
        0x0F | 0x10 => {
            if buf.len() > 4 {
                Some(5 + buf[4] as usize)
            } else {
                // incomplete frame
                None
            }
        }
        0x16 => Some(6),
        0x18 => Some(2),
        0x17 => {
            if buf.len() > 10 {
                Some(9 + buf[10] as usize)
            } else {
                // incomplete frame
                None
            }
        }
        _ => {
            return Err(Error::new(ErrorKind::InvalidData, "invalid data length"));
        }
    };
    Ok(len)
}

fn get_response_payload_len(buf: &BytesMut) -> Result<Option<usize>> {
    if buf.len() < 2 {
        // incomplete frame
        return Ok(None);
    }
    let len = match buf[1] {
        0x01...0x04 | 0x0C | 0x17 => {
            if buf.len() > 2 {
                Some(1 + buf[2] as usize)
            } else {
                // incomplete frame
                None
            }
        }
        0x05 | 0x06 | 0x0B | 0x0F | 0x10 => Some(4),
        0x07 => Some(1),
        0x16 => Some(6),
        0x18 => {
            if buf.len() > 3 {
                Some(2 + Cursor::new(&buf[2..=3]).read_u16::<BigEndian>()? as usize)
            } else {
                // incomplete frame
                None
            }
        }
        0x81...0xAB => Some(1),
        _ => {
            return Err(Error::new(ErrorKind::InvalidData, "invalid data length"));
        }
    };
    Ok(len)
}

fn calc_crc(buf: &[u8]) -> u16 {
    let mut crc = 0xFFFF;
    for x in buf {
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

type ServerAddress = u8;

impl Decoder for RtuDecoder {
    type Item = (ServerAddress, Bytes);
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<(ServerAddress, Bytes)>> {
        let payload_len: Option<usize> = match self.codec_type {
            CodecType::Client => get_response_payload_len(buf)?,
            CodecType::Server => get_request_payload_len(buf)?,
        }.filter(|payload_len| buf.len() >= MIN_ADU_LEN + payload_len);

        if let Some(payload_len) = payload_len {
            let mut adu = buf.split_to(payload_len + 2);
            let crc = buf.split_to(2);
            let crc = Cursor::new(&crc).read_u16::<BigEndian>()?;

            let expected_crc = calc_crc(&adu);
            if expected_crc != crc {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("CRC is not correct: {} instead of {}", crc, expected_crc),
                ));
            }
            let address = adu.split_to(1)[0];
            let data = adu.freeze();
            Ok(Some((address, data)))
        } else {
            // incomplete frame
            Ok(None)
        }
    }
}

impl Decoder for Codec {
    type Item = RtuAdu;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<RtuAdu>> {
        if let Some((address, data)) = self.decoder.decode(buf)? {
            let pdu = match self.decoder.codec_type {
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
            Ok(Some(RtuAdu { address, pdu }))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for Codec {
    type Item = RtuAdu;
    type Error = Error;

    fn encode(&mut self, adu: RtuAdu, buf: &mut BytesMut) -> Result<()> {
        let RtuAdu { address, pdu } = adu;
        let pdu: Bytes = pdu.into();
        buf.reserve(pdu.len() + 3);
        buf.put_u8(address);
        buf.put_slice(&*pdu);
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
    fn test_get_request_payload_len() {
        let mut buf = BytesMut::new();

        buf.extend_from_slice(&[0x66, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(get_request_payload_len(&buf).is_err());

        buf[1] = 0x01;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(4));

        buf[1] = 0x02;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(4));

        buf[1] = 0x03;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(4));

        buf[1] = 0x04;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(4));

        buf[1] = 0x05;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(4));

        buf[1] = 0x06;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(4));

        buf[1] = 0x07;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(0));

        // TODO: 0x08

        buf[1] = 0x0B;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(0));

        buf[1] = 0x0C;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(0));

        buf[1] = 0x0F;
        buf[4] = 99;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(104));

        buf[1] = 0x10;
        buf[4] = 99;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(104));

        buf[1] = 0x11;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(0));

        // TODO: 0x14

        // TODO: 0x15

        buf[1] = 0x16;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(6));

        buf[1] = 0x17;
        buf[10] = 99; // write byte count
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(108));

        buf[1] = 0x18;
        assert_eq!(get_request_payload_len(&buf).unwrap(), Some(2));

        // TODO: 0x2B
    }

    #[test]
    fn test_get_response_payload_len() {

        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x66, 0x01, 99]);
        assert_eq!(get_response_payload_len(&buf).unwrap(),Some(100));

        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x66, 0x00, 99, 0x00]);
        assert!(get_response_payload_len(&buf).is_err());

        buf[1] = 0x01;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(100));

        buf[1] = 0x02;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(100));

        buf[1] = 0x03;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(100));

        buf[1] = 0x04;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(100));

        buf[1] = 0x05;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(4));

        buf[1] = 0x06;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(4));

        buf[1] = 0x07;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(1));

        // TODO: 0x08

        buf[1] = 0x0B;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(4));

        buf[1] = 0x0C;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(100));

        buf[1] = 0x0F;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(4));

        buf[1] = 0x10;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(4));

        // TODO: 0x11

        // TODO: 0x14

        // TODO: 0x15

        buf[1] = 0x16;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(6));

        buf[1] = 0x17;
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(100));

        buf[1] = 0x18;
        buf[2] = 0x01; // byte count Hi
        buf[3] = 0x00; // byte count Lo
        assert_eq!(get_response_payload_len(&buf).unwrap(), Some(258));

        // TODO: 0x2B

        for i in 0x81..0xAB {
            buf[1] = i;
            assert_eq!(get_response_payload_len(&buf).unwrap(), Some(1));
        }
    }

    mod client {

        use super::*;

        #[test]
        fn decode_partly_received_client_message() {
            let mut codec = Codec::client();
            let mut buf = BytesMut::from(vec![
                0x12, // server address
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
        fn decode_partly_received_server_message_0x16() {
            let mut codec = Codec::server();
            let mut buf = BytesMut::from(vec![
                0x12, // server address
                0x16, // function code
                0x00, // irrelevant
                0x00, // irrelevant
            ]);
            assert_eq!(buf.len(), MIN_ADU_LEN);

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(buf.len(), MIN_ADU_LEN);
        }

        #[test]
        fn decode_partly_received_server_message_0x0f() {
            let mut codec = Codec::server();
            let mut buf = BytesMut::from(vec![
                0x12, // server address
                0x0F, // function code
                0x00, // irrelevant
                0x00, // irrelevant
            ]);
            assert_eq!(buf.len(), MIN_ADU_LEN);

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(buf.len(), MIN_ADU_LEN);
        }

        #[test]
        fn decode_partly_received_server_message_0x10() {
            let mut codec = Codec::server();
            let mut buf = BytesMut::from(vec![
                0x12, // server address
                0x10, // function code
                0x00, // irrelevant
                0x00, // irrelevant
            ]);
            assert_eq!(buf.len(), MIN_ADU_LEN);

            let res = codec.decode(&mut buf).unwrap();

            assert!(res.is_none());
            assert_eq!(buf.len(), MIN_ADU_LEN);
        }

        #[test]
        fn decode_rtu_message() {
            let mut codec = Codec::client();
            let mut buf = BytesMut::from(vec![
                0x01, // device address
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
            let RtuAdu { address, pdu } = codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(buf.len(), 1);
            assert_eq!(address, 0x01);
            if let Pdu::Result(res) = pdu {
                if let Response::ReadHoldingRegisters(data) = res.unwrap() {
                    assert_eq!(data.len(), 2);
                    assert_eq!(data, vec![0x8902, 0x42C7]);
                } else {
                    panic!("unexpected response")
                }
            } else {
                panic!("unexpected result")
            }
        }

        #[test]
        fn decode_exception_message() {
            let mut codec = Codec::client();
            let mut buf = BytesMut::from(vec![
                0x66, //
                0x82, // exception = 0x80 + 0x02
                0x03, //
                0xB1, // crc
                0x7E, // crc
            ]);

            let RtuAdu { pdu, .. } = codec.decode(&mut buf).unwrap().unwrap();
            if let Pdu::Result(res) = pdu {
                let err = res.err().unwrap();
                assert_eq!(format!("{}", err), "Modbus function 2: Illegal data value");
            } else {
                panic!("wrong pdu type");
            }
            assert_eq!(buf.len(), 0);
        }

        #[test]
        fn encode_read_request() {
            let mut codec = Codec::client();
            let mut buf = BytesMut::new();
            let req = Request::ReadHoldingRegisters(0x082b, 2);
            let pdu = Pdu::Request(req.clone());
            let address = 0x01;
            let adu = RtuAdu { address, pdu };
            codec.encode(adu.clone(), &mut buf).unwrap();

            assert_eq!(
                buf,
                Bytes::from_static(&[0x01, 0x03, 0x08, 0x2B, 0x00, 0x02, 0xB6, 0x63])
            );
        }

        #[test]
        fn encode_with_limited_buf_capacity() {
            let mut codec = Codec::client();
            let req = Request::ReadHoldingRegisters(0x082b, 2);
            let pdu = Pdu::Request(req.clone());
            let address = 0x01;
            let adu = RtuAdu { address, pdu };
            let mut buf = BytesMut::with_capacity(40);
            unsafe {
                buf.set_len(33);
            }
            assert!(codec.encode(adu, &mut buf).is_ok());
        }
    }
}
