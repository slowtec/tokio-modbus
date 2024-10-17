// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;

use crate::{rtu::RequestContext, ProtocolError, Result, Slave};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Header {
    pub(crate) slave: Slave,
}

#[derive(Debug, Clone)]
pub struct RequestAdu<'a> {
    pub(crate) hdr: Header,
    pub(crate) pdu: RequestPdu<'a>,
}

impl RequestAdu<'_> {
    pub(crate) fn context(&self) -> RequestContext {
        RequestContext {
            function_code: self.pdu.0.function_code(),
            header: self.hdr,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResponseAdu {
    pub(crate) hdr: Header,
    pub(crate) pdu: ResponsePdu,
}

impl ResponseAdu {
    pub(crate) fn try_into_response(self, request_context: RequestContext) -> Result<Response> {
        let RequestContext {
            function_code: req_function_code,
            header: req_hdr,
        } = request_context;

        let ResponseAdu {
            hdr: rsp_hdr,
            pdu: rsp_pdu,
        } = self;
        let ResponsePdu(result) = rsp_pdu;

        if let Err(message) = verify_response_header(&req_hdr, &rsp_hdr) {
            return Err(ProtocolError::HeaderMismatch { message, result }.into());
        }

        // Match function codes of request and response.
        let rsp_function_code = match &result {
            Ok(response) => response.function_code(),
            Err(ExceptionResponse { function, .. }) => *function,
        };
        if req_function_code != rsp_function_code {
            return Err(ProtocolError::FunctionCodeMismatch {
                request: req_function_code,
                result,
            }
            .into());
        }

        Ok(result.map_err(
            |ExceptionResponse {
                 function: _,
                 exception,
             }| exception,
        ))
    }
}

impl<'a> From<RequestAdu<'a>> for Request<'a> {
    fn from(from: RequestAdu<'a>) -> Self {
        from.pdu.into()
    }
}

#[cfg(feature = "server")]
impl<'a> From<RequestAdu<'a>> for SlaveRequest<'a> {
    fn from(from: RequestAdu<'a>) -> Self {
        let RequestAdu { hdr, pdu } = from;
        Self {
            slave: hdr.slave.into(),
            request: pdu.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_same_headers() {
        // Given
        let req_hdr = Header { slave: Slave(0) };
        let rsp_hdr = Header { slave: Slave(0) };

        // When
        let result = verify_response_header(&req_hdr, &rsp_hdr);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_validate_not_same_slave_id() {
        // Given
        let req_hdr = Header { slave: Slave(0) };
        let rsp_hdr = Header { slave: Slave(5) };

        // When
        let result = verify_response_header(&req_hdr, &rsp_hdr);

        // Then
        assert!(result.is_err());
    }
}
