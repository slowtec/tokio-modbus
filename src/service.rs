pub mod tcp {

    use frame::*;
    use proto::tcp::Proto;
    use std::io;
    use std::net::SocketAddr;
    use futures::Future;
    use tokio_core::net::TcpStream;
    use tokio_core::reactor::Handle;
    use tokio_proto::TcpClient;
    use tokio_proto::pipeline::ClientService;
    use tokio_service::Service;

    /// Modbus TCP client
    pub struct Client {
        service: ClientService<TcpStream, Proto>,
    }

    impl Client {
        pub fn connect(
            addr: &SocketAddr,
            handle: &Handle,
        ) -> Box<Future<Item = Client, Error = io::Error>> {
            println!("connect...");
            let ret = TcpClient::new(Proto).connect(addr, handle).map(
                |client_service| Client { service: client_service },
            );
            Box::new(ret)
        }
    }

    impl Service for Client {
        type Request = Request;
        type Response = ModbusResult;
        type Error = io::Error;
        type Future = Box<Future<Item = ModbusResult, Error = io::Error>>;

        fn call(&self, req: Request) -> Self::Future {
            Box::new(self.service.call(req))
        }
    }
}
