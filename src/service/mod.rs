// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
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
