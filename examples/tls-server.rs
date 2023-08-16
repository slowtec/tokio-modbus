// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

// load_certs() and particially load_keys() functions were copied from an example of the tokio tls library, available at:
// https://github.com/tokio-rs/tls/blob/master/tokio-rustls/examples/server/src/main.rs

//! TCP server example

use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader},
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::future;
use pkcs8::der::Decode;
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio::net::{TcpListener, TcpStream};
use tokio_modbus::{prelude::*, server::tcp::Server};
use tokio_rustls::rustls::{self, Certificate, OwnedTrustAnchor, PrivateKey};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use webpki::TrustAnchor;

fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

fn load_keys(path: &Path, password: Option<&str>) -> io::Result<Vec<PrivateKey>> {
    let expected_tag = match &password {
        Some(_) => "ENCRYPTED PRIVATE KEY",
        None => "PRIVATE KEY",
    };

    if expected_tag.eq("PRIVATE KEY") {
        pkcs8_private_keys(&mut BufReader::new(File::open(path)?))
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))
            .map(|mut keys| keys.drain(..).map(PrivateKey).collect())
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
                    let key = rustls::PrivateKey(key);
                    let private_keys = vec![key];
                    io::Result::Ok(private_keys)
                }
                None => io::Result::Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid key")),
            },
            None => io::Result::Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid key")),
        }
    }
}

struct ExampleService {
    input_registers: Arc<Mutex<HashMap<u16, u16>>>,
    holding_registers: Arc<Mutex<HashMap<u16, u16>>>,
}

impl tokio_modbus::server::Service for ExampleService {
    type Request = Request<'static>;
    type Response = Response;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        match req {
            Request::ReadInputRegisters(addr, cnt) => {
                match register_read(&self.input_registers.lock().unwrap(), addr, cnt) {
                    Ok(values) => future::ready(Ok(Response::ReadInputRegisters(values))),
                    Err(err) => future::ready(Err(err)),
                }
            }
            Request::ReadHoldingRegisters(addr, cnt) => {
                match register_read(&self.holding_registers.lock().unwrap(), addr, cnt) {
                    Ok(values) => future::ready(Ok(Response::ReadHoldingRegisters(values))),
                    Err(err) => future::ready(Err(err)),
                }
            }
            Request::WriteMultipleRegisters(addr, values) => {
                match register_write(&mut self.holding_registers.lock().unwrap(), addr, &values) {
                    Ok(_) => future::ready(Ok(Response::WriteMultipleRegisters(
                        addr,
                        values.len() as u16,
                    ))),
                    Err(err) => future::ready(Err(err)),
                }
            }
            Request::WriteSingleRegister(addr, value) => {
                match register_write(
                    &mut self.holding_registers.lock().unwrap(),
                    addr,
                    std::slice::from_ref(&value),
                ) {
                    Ok(_) => future::ready(Ok(Response::WriteSingleRegister(addr, value))),
                    Err(err) => future::ready(Err(err)),
                }
            }
            _ => {
                println!("SERVER: Exception::IllegalFunction - Unimplemented function code in request: {req:?}");
                // TODO: We want to return a Modbus Exception response `IllegalFunction`. https://github.com/slowtec/tokio-modbus/issues/165
                future::ready(Err(std::io::Error::new(
                    std::io::ErrorKind::AddrNotAvailable,
                    "Unimplemented function code in request".to_string(),
                )))
            }
        }
    }
}

impl ExampleService {
    fn new() -> Self {
        // Insert some test data as register values.
        let mut input_registers = HashMap::new();
        input_registers.insert(0, 1234);
        input_registers.insert(1, 5678);
        let mut holding_registers = HashMap::new();
        holding_registers.insert(0, 10);
        holding_registers.insert(1, 20);
        holding_registers.insert(2, 30);
        holding_registers.insert(3, 40);
        Self {
            input_registers: Arc::new(Mutex::new(input_registers)),
            holding_registers: Arc::new(Mutex::new(holding_registers)),
        }
    }
}

/// Helper function implementing reading registers from a HashMap.
fn register_read(
    registers: &HashMap<u16, u16>,
    addr: u16,
    cnt: u16,
) -> Result<Vec<u16>, std::io::Error> {
    let mut response_values = vec![0; cnt.into()];
    for i in 0..cnt {
        let reg_addr = addr + i;
        if let Some(r) = registers.get(&reg_addr) {
            response_values[i as usize] = *r;
        } else {
            // TODO: Return a Modbus Exception response `IllegalDataAddress` https://github.com/slowtec/tokio-modbus/issues/165
            println!("SERVER: Exception::IllegalDataAddress");
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrNotAvailable,
                format!("no register at address {reg_addr}"),
            ));
        }
    }

    Ok(response_values)
}

/// Write a holding register. Used by both the write single register
/// and write multiple registers requests.
fn register_write(
    registers: &mut HashMap<u16, u16>,
    addr: u16,
    values: &[u16],
) -> Result<(), std::io::Error> {
    for (i, value) in values.iter().enumerate() {
        let reg_addr = addr + i as u16;
        if let Some(r) = registers.get_mut(&reg_addr) {
            *r = *value;
        } else {
            // TODO: Return a Modbus Exception response `IllegalDataAddress` https://github.com/slowtec/tokio-modbus/issues/165
            println!("SERVER: Exception::IllegalDataAddress");
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrNotAvailable,
                format!("no register at address {reg_addr}"),
            ));
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket_addr = "127.0.0.1:8802".parse()?;

    tokio::select! {
        _ = server_context(socket_addr) => unreachable!(),
        _ = client_context(socket_addr) => println!("Exiting"),
    }

    Ok(())
}

async fn server_context(socket_addr: SocketAddr) -> anyhow::Result<()> {
    println!("Starting up server on {socket_addr}");
    let listener = TcpListener::bind(socket_addr).await?;
    let server = Server::new(listener);

    let on_connected = |stream, _socket_addr| async move {
        let cert_path = Path::new("./pki/server.pem");
        let key_path = Path::new("./pki/server.key");
        let certs = load_certs(cert_path)?;
        let mut keys = load_keys(key_path, None)?;
        let config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, keys.remove(0))
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        let acceptor = TlsAcceptor::from(Arc::new(config));

        let service = ExampleService::new();
        let stream = acceptor.accept(stream).await;
        match stream {
            Ok(stream) => Ok(Some((service, stream))),
            Err(_) => Ok(None),
        }
    };
    let on_process_error = |err| {
        eprintln!("{err}");
    };
    server.serve(&on_connected, on_process_error).await?;
    Ok(())
}

async fn client_context(socket_addr: SocketAddr) {
    use tokio_modbus::prelude::*;
    tokio::join!(
        async {
            // Give the server some time for starting up
            tokio::time::sleep(Duration::from_secs(1)).await;

            println!("Connecting client...");
            let mut root_cert_store = rustls::RootCertStore::empty();
            let ca_path = Path::new("./pki/ca.pem");
            let mut pem = BufReader::new(File::open(ca_path).unwrap());
            let certs = rustls_pemfile::certs(&mut pem).unwrap();
            let trust_anchors = certs.iter().map(|cert| {
                let ta = TrustAnchor::try_from_cert_der(&cert[..]).unwrap();
                OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                )
            });
            root_cert_store.add_trust_anchors(trust_anchors);

            let domain = "localhost";
            let cert_path = Path::new("./pki/client.pem");
            let key_path = Path::new("./pki/client.key");
            let certs = load_certs(cert_path).unwrap();
            let mut keys = load_keys(key_path, None).unwrap();

            let config = rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_cert_store)
                .with_client_auth_cert(certs, keys.remove(0))
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))
                .unwrap();
            let connector = TlsConnector::from(Arc::new(config));

            let stream = TcpStream::connect(&socket_addr).await.unwrap();
            stream.set_nodelay(true).unwrap();

            let domain = rustls::ServerName::try_from(domain)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))
                .unwrap();

            let transport = connector.connect(domain, stream).await.unwrap();

            // Tokio modbus transport layer setup
            let mut ctx = tcp::attach(transport);

            println!("CLIENT: Reading 2 input registers...");
            let response = ctx.read_input_registers(0x00, 2).await.unwrap();
            println!("CLIENT: The result is '{response:?}'");
            assert_eq!(response, [1234, 5678]);

            println!("CLIENT: Writing 2 holding registers...");
            ctx.write_multiple_registers(0x01, &[7777, 8888])
                .await
                .unwrap();

            // Read back a block including the two registers we wrote.
            println!("CLIENT: Reading 4 holding registers...");
            let response = ctx.read_holding_registers(0x00, 4).await.unwrap();
            println!("CLIENT: The result is '{response:?}'");
            assert_eq!(response, [10, 7777, 8888, 40]);

            // Now we try to read with an invalid register address.
            // This should return a Modbus exception response with the code
            // IllegalDataAddress.
            println!("CLIENT: Reading nonexisting holding register address... (should return IllegalDataAddress)");
            let response = ctx.read_holding_registers(0x100, 1).await;
            println!("CLIENT: The result is '{response:?}'");
            assert!(response.is_err());
            // TODO: How can Modbus client identify Modbus exception responses? E.g. here we expect IllegalDataAddress
            // Question here: https://github.com/slowtec/tokio-modbus/issues/169

            println!("CLIENT: Done.")
        },
        tokio::time::sleep(Duration::from_secs(5))
    );
}
