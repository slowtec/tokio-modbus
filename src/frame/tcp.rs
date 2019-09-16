use super::*;

pub(crate) type TransactionId = u16;
pub(crate) type UnitId = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Header {
    pub transaction_id: TransactionId,
    pub unit_id: UnitId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestAdu {
    pub hdr: Header,
    pub pdu: RequestPdu,
    pub disconnect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResponseAdu {
    pub hdr: Header,
    pub pdu: ResponsePdu,
}
