// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;

pub(crate) type TransactionId = u16;
pub(crate) type UnitId = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Header {
    pub(crate) transaction_id: TransactionId,
    pub(crate) unit_id: UnitId,
}

#[derive(Debug, Clone)]
pub struct RequestAdu<'a> {
    pub(crate) hdr: Header,
    pub(crate) pdu: RequestPdu<'a>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResponseAdu {
    pub(crate) hdr: Header,
    pub(crate) pdu: ResponsePdu,
}

impl<'a> From<RequestAdu<'a>> for Request<'a> {
    fn from(from: RequestAdu<'a>) -> Self {
        from.pdu.into()
    }
}

#[cfg(feature = "server")]
impl<'a> From<RequestAdu<'a>> for SlaveRequest<'a> {
    fn from(from: RequestAdu<'a>) -> Self {
        Self {
            slave: from.hdr.unit_id,
            request: from.pdu.into(),
        }
    }
}
