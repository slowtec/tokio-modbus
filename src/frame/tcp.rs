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

impl VerifiableHeader for Header {
    /// Verify that the response is from the correct transaction,
    /// and that the responder's UnitID is valid
    //
    // OXP1 breaks the ModBus specification:
    // "
    // 4.4.2.5: Server protocol, Response building:
    // Unit Identifier
    // The Unit Identifier is copied as it was given within the received MODBUS request
    // and memorized in the transaction context.
    // "
    //
    // OXP1 always returns `unit_id=1`, and we as a client are forced (by spec)
    // to feed it `unit_id=255`.
    //
    // If OXP1 was conformant, then the base `tokio-modbus` functionality
    // would suit just fine
    fn verify_against(&self, rsp: &Self) -> Result<(), String> {
        if self.transaction_id != rsp.transaction_id {
            return Err(format!(
                "mismatched transaction ID: request = {self:?}, response = {rsp:?}"
            ));
        }
        Ok(())
    }
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
