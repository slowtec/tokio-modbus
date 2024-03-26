// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(feature = "rtu")]
pub(crate) mod rtu;

#[cfg(feature = "tcp")]
pub(crate) mod tcp;

/// Check that `req_hdr` is the same `Header` as `rsp_hdr`.
///
/// # Errors
///
/// If the 2 headers are different, an error message with the details will be returned.
#[cfg(any(feature = "rtu", feature = "tcp"))]
fn verify_response_header<H: Eq + std::fmt::Debug>(req_hdr: &H, rsp_hdr: &H) -> Result<(), String> {
    if req_hdr != rsp_hdr {
        return Err(format!(
            "expected/request = {req_hdr:?}, actual/response = {rsp_hdr:?}"
        ));
    }
    Ok(())
}
