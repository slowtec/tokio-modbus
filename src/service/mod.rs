// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
