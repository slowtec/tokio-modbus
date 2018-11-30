#[cfg(feature = "tcp")]
pub mod tcp;

use crate::frame::*;

use futures::prelude::*;
use std::io::Error;
use tokio_service::Service;

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
