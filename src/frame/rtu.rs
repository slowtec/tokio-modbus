use super::*;

use crate::slave::SlaveId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Header {
    pub slave_id: SlaveId,
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
