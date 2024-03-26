// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Error types.

use thiserror::Error;

use crate::{Exception, ExceptionResponse, FunctionCode, Response};

/// Error type for _Modbus_ responses.
#[derive(Debug, Error)]
pub enum ResponseError {
    /// The server responded with a _Modbus_ exception.
    #[error("exception: {0}")]
    Exception(#[from] Exception),

    /// The received response header doesn't match the request.
    ///
    /// The error message contains details about the mismatch.
    ///
    /// The result received from the server is included for further analysis and handling.
    #[error("mismatching headers: {message} {result:?}")]
    MismatchingHeaders {
        message: String,
        result: Result<Response, ExceptionResponse>,
    },

    /// The received response function code doesn't match the request.
    ///
    /// The result received from the server is included for further analysis and handling.
    #[error("mismatching function codes: {request} {result:?}")]
    MismatchingFunctionCodes {
        request: FunctionCode,
        result: Result<Response, ExceptionResponse>,
    },
}
