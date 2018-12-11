use super::*;

pub(crate) type SlaveAddress = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Header {
    pub slave_addr: SlaveAddress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestAdu {
    pub hdr: Header,
    pub pdu: RequestPdu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResponseAdu {
    pub hdr: Header,
    pub pdu: ResponsePdu,
}
