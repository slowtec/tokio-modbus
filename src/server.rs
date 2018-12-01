use crate::frame::*;
use futures::prelude::*;
use crate::proto;
use std::io::Error;
use std::net::SocketAddr;
use tokio_proto::TcpServer;
use tokio_service::{NewService, Service};

/// A multithreaded Modbus server.
pub struct Server {
    server_type: ServerType,
}

enum ServerType {
    Tcp(SocketAddr),
}

struct ServiceWrapper<S> {
    service: S,
}

impl<S> ServiceWrapper<S> {
    fn new(service: S) -> ServiceWrapper<S> {
        ServiceWrapper { service }
    }
}

impl<S> Service for ServiceWrapper<S>
where
    S: Service + Send + Sync + 'static,
    S::Request: From<Request>,
    S::Response: Into<Response>,
    S::Error: Into<Error>,
{
    type Request = TcpAdu;
    type Response = TcpAdu;
    type Error = Error;
    type Future = Box<Future<Item = Self::Request, Error = Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let TcpAdu { header, pdu } = req;
        if let Pdu::Request(req) = pdu {
            Box::new(self.service.call(req.into()).then(|res| match res {
                Ok(res) => {
                    let pdu = Pdu::Result(Ok(res.into()));
                    Ok(TcpAdu { header, pdu })
                }
                Err(e) => Err(e.into()),
            }))
        } else {
            panic!("Received response instead of a request");
        }
    }
}

impl Server {
    /// Create a new Modbus TCP server instance.
    #[cfg(feature = "tcp")]
    pub fn new_tcp(addr: SocketAddr) -> Server {
        Server {
            server_type: ServerType::Tcp(addr),
        }
    }

    #[cfg(feature = "tcp")]
    pub fn serve<S>(&self, service: S)
    where
        S: NewService + Send + Sync + 'static,
        S::Request: From<Request>,
        S::Response: Into<Response>,
        S::Error: Into<Error>,
        S::Instance: Send + Sync + 'static,
    {
        match self.server_type {
            ServerType::Tcp(addr) => {
                TcpServer::new(proto::tcp::Proto, addr)
                    .serve(move || Ok(ServiceWrapper::new(service.new_service()?)));
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use futures::future;

    #[test]
    fn service_wrapper() {
        #[derive(Clone)]
        struct DummyService {
            response: Response,
        };

        impl Service for DummyService {
            type Request = Request;
            type Response = Response;
            type Error = Error;
            type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

            fn call(&self, _: Self::Request) -> Self::Future {
                Box::new(future::ok(self.response.clone()))
            }
        }

        let s = DummyService {
            response: Response::ReadInputRegisters(vec![0x33]),
        };
        let service = ServiceWrapper::new(s.clone());
        let pdu = Pdu::Request(Request::ReadInputRegisters(0, 1));
        let header = TcpHeader {
            transaction_id: 9,
            unit_id: 7,
        };
        let adu = TcpAdu { header, pdu };
        let res = service.call(adu).wait().unwrap();
        assert_eq!(
            res.header,
            TcpHeader {
                transaction_id: 9,
                unit_id: 7,
            }
        );
        assert_eq!(res.pdu, Pdu::Result(Ok(s.response)));
    }
}
