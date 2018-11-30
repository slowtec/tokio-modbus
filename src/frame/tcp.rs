use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Header {
    pub transaction_id: u16,
    pub unit_id: u8,
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
