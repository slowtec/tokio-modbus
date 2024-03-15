// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus Error helpers.

use crate::FunctionCode;

/// Message to show when a bug has been found during runtime execution.
const REPORT_ISSUE_MSG: &str =
    "Please report the issue at `https://github.com/slowtec/tokio-modbus/issues` with a minimal example reproducing this bug.";

/// Create a panic message for `unexpected response code` with `req_code` and `rsp_code`.
pub(crate) fn unexpected_rsp_code_panic_msg(
    req_code: FunctionCode,
    rsp_code: FunctionCode,
) -> String {
    format!(
        "unexpected response code: {rsp_code} (request code: {req_code})\nnote: {REPORT_ISSUE_MSG}"
    )
}
