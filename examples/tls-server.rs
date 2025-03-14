// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

// load_certs() and particially load_keys() functions were copied from an example of the tokio tls library, available at:
// https://github.com/tokio-rs/tls/blob/master/tokio-rustls/examples/server/src/main.rs

//! TCP server example

use std::{
    collections::HashMap,
    fs::File,
    future,
    io::{self, BufReader},
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use pkcs8::der::Decode;
use pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio::net::{TcpListener, TcpStream};
use tokio_modbus::{prelude::*, server::tcp::Server};
use tokio_rustls::{TlsAcceptor, TlsConnector};

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

struct ExampleService {
    input_registers: Arc<Mutex<HashMap<u16, u16>>>,
    holding_registers: Arc<Mutex<HashMap<u16, u16>>>,
}

impl tokio_modbus::server::Service for ExampleService {
    type Request = Request<'static>;
    type Response = Response;
    type Exception = ExceptionCode;
    type Future = future::Ready<Result<Self::Response, Self::Exception>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let res = match req {
            Request::ReadInputRegisters(addr, cnt) => {
                register_read(&self.input_registers.lock().unwrap(), addr, cnt)
                    .map(Response::ReadInputRegisters)
            }
            Request::ReadHoldingRegisters(addr, cnt) => {
                register_read(&self.holding_registers.lock().unwrap(), addr, cnt)
                    .map(Response::ReadHoldingRegisters)
            }
            Request::WriteMultipleRegisters(addr, values) => {
                register_write(&mut self.holding_registers.lock().unwrap(), addr, &values)
                    .map(|_| Response::WriteMultipleRegisters(addr, values.len() as u16))
            }
            Request::WriteSingleRegister(addr, value) => register_write(
                &mut self.holding_registers.lock().unwrap(),
                addr,
                std::slice::from_ref(&value),
            )
            .map(|_| Response::WriteSingleRegister(addr, value)),
            _ => {
                println!("SERVER: Exception::IllegalFunction - Unimplemented function code in request: {req:?}");
                Err(ExceptionCode::IllegalFunction)
            }
        };
        future::ready(res)
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
) -> Result<Vec<u16>, ExceptionCode> {
    let mut response_values = vec![0; cnt.into()];
    for i in 0..cnt {
        let reg_addr = addr + i;
        if let Some(r) = registers.get(&reg_addr) {
            response_values[i as usize] = *r;
        } else {
            println!("SERVER: Exception::IllegalDataAddress");
            return Err(ExceptionCode::IllegalDataAddress);
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
) -> Result<(), ExceptionCode> {
    for (i, value) in values.iter().enumerate() {
        let reg_addr = addr + i as u16;
        if let Some(r) = registers.get_mut(&reg_addr) {
            *r = *value;
        } else {
            println!("SERVER: Exception::IllegalDataAddress");
            return Err(ExceptionCode::IllegalDataAddress);
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
        let key = load_keys(key_path, None)?;
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
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
            let certs = rustls_pemfile::certs(&mut pem)
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            root_cert_store.add_parsable_certificates(certs);

            let domain = "localhost";
            let cert_path = Path::new("./pki/client.pem");
            let key_path = Path::new("./pki/client.key");
            let certs = load_certs(cert_path).unwrap();
            let key = load_keys(key_path, None).unwrap();

            let config = rustls::ClientConfig::builder()
                .with_root_certificates(root_cert_store)
                .with_client_auth_cert(certs, key)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))
                .unwrap();
            let connector = TlsConnector::from(Arc::new(config));

            let stream = TcpStream::connect(&socket_addr).await.unwrap();
            stream.set_nodelay(true).unwrap();

            let domain = ServerName::try_from(domain)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))
                .unwrap();

            let transport = connector.connect(domain, stream).await.unwrap();

            // Tokio modbus transport layer setup
            let mut ctx = tcp::attach(transport);

            println!("CLIENT: Reading 2 input registers...");
            let response = ctx.read_input_registers(0x00, 2).await.unwrap();
            println!("CLIENT: The result is '{response:?}'");
            assert_eq!(response.unwrap(), vec![1234, 5678]);

            println!("CLIENT: Writing 2 holding registers...");
            ctx.write_multiple_registers(0x01, &[7777, 8888])
                .await
                .unwrap()
                .unwrap();

            // Read back a block including the two registers we wrote.
            println!("CLIENT: Reading 4 holding registers...");
            let response = ctx.read_holding_registers(0x00, 4).await.unwrap();
            println!("CLIENT: The result is '{response:?}'");
            assert_eq!(response.unwrap(), vec![10, 7777, 8888, 40]);

            // Now we try to read with an invalid register address.
            // This should return a Modbus exception response with the code
            // IllegalDataAddress.
            println!("CLIENT: Reading nonexistent holding register address... (should return IllegalDataAddress)");
            let response = ctx.read_holding_registers(0x100, 1).await.unwrap();
            println!("CLIENT: The result is '{response:?}'");
            assert!(matches!(response, Err(ExceptionCode::IllegalDataAddress)));

            println!("CLIENT: Done.")
        },
        tokio::time::sleep(Duration::from_secs(5))
    );
}
