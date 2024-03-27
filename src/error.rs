// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Error types.

use thiserror::Error;

use crate::{ExceptionResponse, FunctionCode, Response};

/// Protocol or transport errors.
///
/// Devices that don't implement the _Modbus_ protocol correctly
/// or network issues can cause these errors.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    Transport(#[from] std::io::Error),
}

/// _Modbus_ protocol error.
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// The received response header doesn't match the request.
    ///
    /// The error message contains details about the mismatch.
    ///
    /// The result received from the server is included for further analysis and handling.
    #[error("mismatching headers: {message} {result:?}")]
    HeaderMismatch {
        message: String,
        result: Result<Response, ExceptionResponse>,
    },

    /// The received response function code doesn't match the request.
    ///
    /// The result received from the server is included for further analysis and handling.
    #[error("mismatching function codes: {request} {result:?}")]
    FunctionCodeMismatch {
        request: FunctionCode,
        result: Result<Response, ExceptionResponse>,
    },
}
