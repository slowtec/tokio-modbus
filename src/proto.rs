pub mod tcp {

    use frame::TcpAdu;
    use tokio_io::{AsyncRead, AsyncWrite};
    use std::io::Error;
    use tokio_io::codec::Framed;
    use tokio_proto::pipeline::ClientProto;
    use codec::tcp::ClientCodec;

    pub struct Proto;

    impl<T: AsyncRead + AsyncWrite + 'static> ClientProto<T> for Proto {
        type Request = TcpAdu;
        type Response = TcpAdu;
        type Transport = Framed<T, ClientCodec>;
        type BindTransport = Result<Self::Transport, Error>;

        fn bind_transport(&self, io: T) -> Self::BindTransport {
            Ok(io.framed(ClientCodec::new()))
        }
    }
}
