// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

// load_certs() and partially load_keys() functions were copied from an example of the tokio tls library, available at:
// https://github.com/tokio-rs/tls/blob/master/tokio-rustls/examples/server/src/main.rs

//! Asynchronous TLS client example
use tokio::net::TcpStream;

use std::{
    fs::File,
    io::{self, BufReader},
    net::SocketAddr,
    path::Path,
    sync::Arc,
};

use pkcs8::der::Decode;
use pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::TlsConnector;

fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    certs(&mut BufReader::new(File::open(path)?)).collect()
}

fn load_keys(path: &Path, password: Option<&str>) -> io::Result<PrivateKeyDer<'static>> {
    let expected_tag = match &password {
        Some(_) => "ENCRYPTED PRIVATE KEY",
        None => "PRIVATE KEY",
    };

    if expected_tag.eq("PRIVATE KEY") {
        pkcs8_private_keys(&mut BufReader::new(File::open(path)?))
            .next()
            .unwrap()
            .map(Into::into)
    } else {
        let content = std::fs::read(path)?;
        let mut iter = pem::parse_many(content)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?
            .into_iter()
            .filter(|x| x.tag() == expected_tag)
            .map(|x| x.contents().to_vec());

        match iter.next() {
            Some(key) => match password {
                Some(password) => {
                    let encrypted =
                        pkcs8::EncryptedPrivateKeyInfo::from_der(&key).map_err(|err| {
                            io::Error::new(io::ErrorKind::InvalidData, err.to_string())
                        })?;
                    let decrypted = encrypted.decrypt(password).map_err(|err| {
                        io::Error::new(io::ErrorKind::InvalidData, err.to_string())
                    })?;
                    let key = decrypted.as_bytes().to_vec();
                    match rustls_pemfile::read_one_from_slice(&key)
                        .expect("cannot parse private key .pem file")
                    {
                        Some((rustls_pemfile::Item::Pkcs1Key(key), _keys)) => {
                            io::Result::Ok(key.into())
                        }
                        Some((rustls_pemfile::Item::Pkcs8Key(key), _keys)) => {
                            io::Result::Ok(key.into())
                        }
                        Some((rustls_pemfile::Item::Sec1Key(key), _keys)) => {
                            io::Result::Ok(key.into())
                        }
                        _ => io::Result::Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "invalid key",
                        )),
                    }
                }
                None => io::Result::Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid key")),
            },
            None => io::Result::Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid key")),
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tokio_modbus::prelude::*;

    let socket_addr: SocketAddr = "127.0.0.1:8802".parse()?;

    let mut root_cert_store = tokio_rustls::rustls::RootCertStore::empty();
    let ca_path = Path::new("./pki/ca.pem");
    let mut pem = BufReader::new(File::open(ca_path)?);
    let certs = rustls_pemfile::certs(&mut pem).collect::<Result<Vec<_>, _>>()?;
    root_cert_store.add_parsable_certificates(certs);

    let domain = "localhost";
    let cert_path = Path::new("./pki/client.pem");
    let key_path = Path::new("./pki/client.key");
    let certs = load_certs(cert_path)?;
    let key = load_keys(key_path, None)?;

    let config = tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_client_auth_cert(certs, key)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    let connector = TlsConnector::from(Arc::new(config));

    let stream = TcpStream::connect(&socket_addr).await?;
    stream.set_nodelay(true)?;

    let domain = ServerName::try_from(domain)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;

    let transport = connector.connect(domain, stream).await?;

    // Tokio modbus transport layer setup
    let mut ctx = tcp::attach(transport);

    println!("Reading Holding Registers");
    let data = ctx.read_holding_registers(40000, 68).await?;
    println!("Holding Registers Data is '{data:?}'");
    ctx.disconnect().await?;

    Ok(())
}
