use crate::{
    codec,
    frame::*,
    server::service::{NewService, Service},
};
use std::{
    io::{Error},
    path::Path
};
use tokio_util::codec::Framed;
use futures::{select, Future, FutureExt};
use futures_util::{StreamExt, SinkExt};
use tokio_serial::{Serial, SerialPortSettings};

pub struct Server {
    serial: Serial
}
impl Server {
    pub fn new_from_path<P: AsRef<Path>>(p: P, settings: &SerialPortSettings) -> Result<Self, Error> {
        let serial = Serial::from_path(p, settings)?;
        Ok(Server {
            serial
        })
    }
    pub fn new(serial: Serial) -> Self {
        Server {
            serial
        }
    }

    pub fn serve_forever<S>(self, new_service: S)
        where
            S: NewService<Request = Request, Response = Response> + Send + Sync + 'static,
            S::Error: Into<Error>,
            S::Instance: 'static + Send + Sync,
    {
        self.serve_until(new_service, futures::future::pending())
    }

    pub fn serve_until<S, Sd>(self, new_service: S, shutdown_signal: Sd)
    where
        S: NewService<Request = Request, Response = Response> + Send + Sync + 'static,
        Sd: Future<Output = ()> + Sync + Send + Unpin + 'static,
        S::Request: From<Request>,
        S::Response: Into<Response>,
        S::Error: Into<Error>,
        S::Instance: Send + Sync + 'static,
    {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let framed = Framed::new(self.serial, codec::rtu::ServerCodec::default());
        let service = new_service.new_service().unwrap();
        let future = process(framed, service);

        let mut server = Box::pin(future).fuse();
        let mut shutdown = shutdown_signal.fuse();

        let task = async {
            select!{
                res = server => match res {
                    Err(e) => println!("error: {}", e),
                    _ => {}
                },
                _ = shutdown => { println!("Shutdown signal received")  }
            }
        };

        rt.block_on(task);
    }
}

async fn process<S>(
    mut framed: Framed<Serial, codec::rtu::ServerCodec>,
    service: S,
) -> Result<(), Error>
    where
        S: Service<Request = Request, Response = Response> + Send + Sync + 'static,
        S::Error: Into<Error>,
{
    loop {
        let request = framed.next().await;
        if request.is_none() {
            break;
        }
        let request = request.unwrap()?;
        let hdr = request.hdr;
        let response = service.call(request.pdu.0).await.map_err(Into::into)?;
        framed.send(
            rtu::ResponseAdu {
                hdr,
                pdu: response.into(),
            }
        ).await?;
    }
    Ok(())
}
