// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Asynchronous TLS client example
use tokio::net::TcpStream;

use std::{
    fs::File,
    io::{self, BufReader},
    net::SocketAddr,
    path::Path,
    sync::Arc
};

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tokio_modbus::prelude::*;

    let socket_addr: SocketAddr = "127.0.0.1:8802".parse().unwrap();

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

    let transport = connector.connect(domain, stream).await?;
    let mut ctx = tcp::attach(transport);


    println!("Reading Holding Registers");
    let data = ctx.read_holding_registers(40000, 68).await?;

    println!("Holding Registers Data is '{:?}'", data);

    Ok(())
}
