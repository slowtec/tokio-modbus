// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;

use crate::slave::SlaveId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Header {
    pub(crate) slave_id: SlaveId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestAdu<'a> {
    pub(crate) hdr: Header,
    pub(crate) pdu: RequestPdu<'a>,
    pub(crate) disconnect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
            slave: from.hdr.slave_id,
            request: from.pdu.into(),
        }
    }
}
