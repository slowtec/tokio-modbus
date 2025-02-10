// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::frame::VerifiableHeader;

#[cfg(feature = "rtu")]
pub(crate) mod rtu;

#[cfg(feature = "tcp")]
pub(crate) mod tcp;

#[cfg(any(feature = "rtu", feature = "tcp"))]
async fn disconnect<T, C>(framed: tokio_util::codec::Framed<T, C>) -> std::io::Result<()>
where
    T: tokio::io::AsyncWrite + Unpin,
{
    use tokio::io::AsyncWriteExt as _;

    framed
        .into_inner()
        .shutdown()
        .await
        .or_else(|err| match err.kind() {
            std::io::ErrorKind::NotConnected | std::io::ErrorKind::BrokenPipe => {
                // Already disconnected.
                Ok(())
            }
            _ => Err(err),
        })
}

/// Check that `rsp_hdr` is valid for a given `req_hdr`, according to the
/// specific protocol's specification
///
/// # Errors
///
/// If the response header is not valid, returns an error message with the details.
#[cfg(any(feature = "rtu", feature = "tcp"))]
fn verify_response_header<H: VerifiableHeader>(req_hdr: &H, rsp_hdr: &H) -> Result<(), String> {
    req_hdr.verify_against(rsp_hdr)
}
