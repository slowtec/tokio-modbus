use frame::*;
use std::io::{Error, Result};
use bytes::{BigEndian, BufMut, Bytes, BytesMut};
use tokio_io::codec;
const TCP_PROTOCOL_ID: u16 = 0x0;

pub struct Encoder;

impl codec::Encoder for Encoder {
    type Item = Adu;
    type Error = Error;

    fn encode(&mut self, adu: Adu, buf: &mut BytesMut) -> Result<()> {
        match adu {
            Adu::Tcp(header, pdu) => {
                let pdu: Bytes = pdu.into();
                buf.put_u16::<BigEndian>(header.transaction_id);
                buf.put_u16::<BigEndian>(TCP_PROTOCOL_ID);
                buf.put_u16::<BigEndian>((pdu.len() + 1) as u16);
                buf.put_u8(header.unit_id);
                buf.extend_from_slice(&*pdu);
            }
        }
        Ok(())
    }
}
