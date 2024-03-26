// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Error types.

use thiserror::Error;

use crate::{Exception, Response};

/// Error type for _Modbus_ responses.
#[derive(Debug, Error)]
pub enum ResponseError {
    /// The server responded with a _Modbus_ exception.
    #[error("exception: {0}")]
    Exception(#[from] Exception),

    /// The received response doesn't match the request.
    ///
    /// This happens if the _Modbus_ function codes of the request and response do not match.
    #[error("unexpected response: {response:?}")]
    UnexpectedResponse { response: Response },
}
