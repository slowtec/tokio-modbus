// SPDX-FileCopyrightText: Copyright (c) 2017-2026 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

// load_certs() and partially load_keys() functions were copied from an example of the tokio tls library, available at:
// https://github.com/tokio-rs/tls/blob/master/tokio-rustls/examples/server/src/main.rs

//! Asynchronous TLS client example
use anyhow::{bail, Context};
use tokio::net::TcpStream;

use std::{fs::File, io::BufReader, net::SocketAddr, path::Path, sync::Arc};

use pkcs8::der::Decode;
use rustls_pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer, ServerName};
use tokio_rustls::TlsConnector;

fn load_certs(path: &Path) -> anyhow::Result<Vec<CertificateDer<'static>>> {
    let mut reader = BufReader::new(File::open(path)?);
    Ok(CertificateDer::pem_reader_iter(&mut reader).collect::<Result<_, _>>()?)
}

fn load_keys(path: &Path, password: Option<&str>) -> anyhow::Result<PrivateKeyDer<'static>> {
    let expected_tag = match &password {
        Some(_) => "ENCRYPTED PRIVATE KEY",
        None => "PRIVATE KEY",
    };

    if expected_tag.eq("PRIVATE KEY") {
        let mut reader = BufReader::new(File::open(path)?);
        let key = PrivateKeyDer::from_pem_reader(&mut reader)?;
        return Ok(key);
    }
    let content = std::fs::read(path)?;
    let mut iter = pem::parse_many(content)?
        .into_iter()
        .filter(|x| x.tag() == expected_tag)
        .map(|x| x.contents().to_vec());

    match iter.next() {
        Some(key) => match password {
            Some(password) => {
                let encrypted = pkcs8::EncryptedPrivateKeyInfo::from_der(&key)?;
                let decrypted = encrypted.decrypt(password)?;
                let bytes = decrypted.as_bytes().to_vec();
                Ok(PrivateKeyDer::from_pem_slice(&bytes)
                    .expect("cannot parse private key .pem file"))
            }
            None => bail!("invalid key"),
        },
        None => bail!("invalid key"),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tokio_modbus::prelude::*;

    let socket_addr: SocketAddr = "127.0.0.1:8802".parse()?;

    let mut root_cert_store = tokio_rustls::rustls::RootCertStore::empty();
    let ca_path = Path::new("./pki/ca.pem");
    let mut pem = BufReader::new(File::open(ca_path)?);
    let certs = CertificateDer::pem_reader_iter(&mut pem).collect::<Result<Vec<_>, _>>()?;
    root_cert_store.add_parsable_certificates(certs);

    let domain = "localhost";
    let cert_path = Path::new("./pki/client.pem");
    let key_path = Path::new("./pki/client.key");
    let certs = load_certs(cert_path)?;
    let key = load_keys(key_path, None)?;

    let config = tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_client_auth_cert(certs, key)?;
    let connector = TlsConnector::from(Arc::new(config));

    let stream = TcpStream::connect(&socket_addr).await?;
    stream.set_nodelay(true)?;

    let domain = ServerName::try_from(domain).context("invalid dnsname")?;

    let transport = connector.connect(domain, stream).await?;

    // Tokio modbus transport layer setup
    let mut ctx = tcp::attach(transport);

    println!("Reading Holding Registers");
    let data = ctx.read_holding_registers(40000, 68).await?;
    println!("Holding Registers Data is '{data:?}'");
    ctx.disconnect().await?;

    Ok(())
}
