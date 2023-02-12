// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;

use crate::slave::SlaveId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Header {
    pub(crate) slave_id: SlaveId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestAdu {
    pub(crate) hdr: Header,
    pub(crate) pdu: RequestPdu,
    pub(crate) disconnect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResponseAdu {
    pub(crate) hdr: Header,
    pub(crate) pdu: ResponsePdu,
}

impl From<RequestAdu> for Request {
    fn from(from: RequestAdu) -> Self {
        from.pdu.into()
    }
}

#[cfg(feature = "server")]
impl From<RequestAdu> for SlaveRequest {
    fn from(from: RequestAdu) -> Self {
        Self {
            slave: from.hdr.slave_id,
            request: from.pdu.into(),
        }
    }
}
