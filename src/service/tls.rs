// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    client::Client,
    codec,
    frame::{tcp::*, *},
    slave::*,
};

use futures_util::{sink::SinkExt as _, stream::StreamExt as _};
use std::{
    convert::From,
    fs::File,
    future::Future,
    io::{self, BufReader, Error, ErrorKind},
    net::SocketAddr,
    path::Path,
    sync::atomic::{AtomicU16, Ordering},
    sync::Arc,
};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::rustls::{self, Certificate, OwnedTrustAnchor, PrivateKey};
use tokio_rustls::{webpki, TlsConnector};

fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
    pkcs8_private_keys(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))
        .map(|mut keys| keys.drain(..).map(PrivateKey).collect())
}

pub(crate) fn connect_slave(
    socket_addr: SocketAddr,
    slave: Slave,
) -> impl Future<Output = Result<Context, Error>> + 'static {
    let unit_id: UnitId = slave.into();
    async move {
        let mut root_cert_store = rustls::RootCertStore::empty();
        let ca_path = Path::new("./pki/ca.pem");
        let mut pem = BufReader::new(File::open(ca_path)?);
        let certs = rustls_pemfile::certs(&mut pem)?;
        let trust_anchors = certs.iter().map(|cert| {
            let ta = webpki::TrustAnchor::try_from_cert_der(&cert[..]).unwrap();
            OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        });
        root_cert_store.add_server_trust_anchors(trust_anchors);

        let domain = "localhost";
        let cert_path = Path::new("./pki/client.pem");
        let key_path = Path::new("./pki/client.key");
        let certs = load_certs(cert_path)?;
        let mut keys = load_keys(key_path)?;

        let config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_cert_store)
            .with_single_cert(certs, keys.remove(0))
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        let connector = TlsConnector::from(Arc::new(config));

        let stream = TcpStream::connect(&socket_addr).await?;

        let domain = rustls::ServerName::try_from(domain)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;
    
        let service = connector.connect(domain, stream).await?;

        let framed = Framed::new(service, codec::tcp::ClientCodec::default());

        let context: Context = Context::new(framed, unit_id);

        Ok(context)
    }
}

const INITIAL_TRANSACTION_ID: TransactionId = 0;

/// Modbus TLS client
#[derive(Debug)]
pub(crate) struct Context {
    service: Framed<tokio_rustls::client::TlsStream<TcpStream>, codec::tcp::ClientCodec>,
    unit_id: UnitId,
    transaction_id: AtomicU16,
}

impl Context {
    fn new(
        service: Framed<tokio_rustls::client::TlsStream<TcpStream>, codec::tcp::ClientCodec>,
        unit_id: UnitId,
    ) -> Self {
        Self {
            service,
            unit_id,
            transaction_id: AtomicU16::new(INITIAL_TRANSACTION_ID),
        }
    }

    fn next_transaction_id(&self) -> TransactionId {
        let transaction_id = self.transaction_id.load(Ordering::Relaxed);
        self.transaction_id
            .store(transaction_id.wrapping_add(1), Ordering::Relaxed);
        transaction_id
    }

    fn next_request_hdr(&self, unit_id: UnitId) -> Header {
        let transaction_id = self.next_transaction_id();
        Header {
            transaction_id,
            unit_id,
        }
    }

    fn next_request_adu<R>(&self, req: R, disconnect: bool) -> RequestAdu
    where
        R: Into<RequestPdu>,
    {
        RequestAdu {
            hdr: self.next_request_hdr(self.unit_id),
            pdu: req.into(),
            disconnect,
        }
    }

    pub(crate) async fn call(&mut self, req: Request) -> Result<Response, Error> {
        log::debug!("Call {:?}", req);
        let disconnect = req == Request::Disconnect;
        let req_adu = self.next_request_adu(req, disconnect);
        let req_hdr = req_adu.hdr;

        self.service.send(req_adu).await?;
        let res_adu = self
            .service
            .next()
            .await
            .ok_or_else(Error::last_os_error)??;

        match res_adu.pdu {
            ResponsePdu(Ok(res)) => verify_response_header(req_hdr, res_adu.hdr).and(Ok(res)),
            ResponsePdu(Err(err)) => Err(Error::new(ErrorKind::Other, err)),
        }
    }
}

fn verify_response_header(req_hdr: Header, rsp_hdr: Header) -> Result<(), Error> {
    if req_hdr != rsp_hdr {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Invalid response header: expected/request = {req_hdr:?}, actual/response = {rsp_hdr:?}"
            ),
        ));
    }
    Ok(())
}

impl SlaveContext for Context {
    fn set_slave(&mut self, slave: Slave) {
        self.unit_id = slave.into();
    }
}

#[async_trait::async_trait]
impl Client for Context {
    async fn call(&mut self, req: Request) -> Result<Response, Error> {
        Context::call(self, req).await
    }
}
