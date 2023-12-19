// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Library Error type

use std::fmt::Display;

use crate::frame::ExceptionResponse;

/// A specialized [Result] type for Modbus operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Modbus errors.
#[derive(Debug)]
pub enum Error {
    /// Modbus Exception.
    Exception(ExceptionResponse),

    /// General I/O Error,
    Io(std::io::Error),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exception(rsp) => rsp.fmt(f),
            Self::Io(io) => io.fmt(f),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<ExceptionResponse> for Error {
    fn from(value: ExceptionResponse) -> Self {
        Error::Exception(value)
    }
}
