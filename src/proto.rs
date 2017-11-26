pub mod tcp {

    use frame::TcpAdu;
    use tokio_io::{AsyncRead, AsyncWrite};
    use std::io::Error;
    use tokio_io::codec::Framed;
    use tokio_proto::pipeline::{ClientProto, ServerProto};
    use codec::tcp::Codec;

    pub struct Proto;

    impl<T: AsyncRead + AsyncWrite + 'static> ClientProto<T> for Proto {
        type Request = TcpAdu;
        type Response = TcpAdu;
        type Transport = Framed<T, Codec>;
        type BindTransport = Result<Self::Transport, Error>;

        fn bind_transport(&self, io: T) -> Self::BindTransport {
            Ok(io.framed(Codec::client()))
        }
    }

    impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for Proto {
        type Request = TcpAdu;
        type Response = TcpAdu;
        type Transport = Framed<T, Codec>;
        type BindTransport = Result<Self::Transport, Error>;

        fn bind_transport(&self, io: T) -> Self::BindTransport {
            Ok(io.framed(Codec::server()))
        }
    }
}
