#[cfg(feature = "tcp")]
pub mod tcp {

    use frame::TcpAdu;
    use tokio_io::{AsyncRead, AsyncWrite};
    use std::io::Error;
    use tokio_io::codec::Framed;
    use tokio_proto::pipeline::{ClientProto, ServerProto};
    use codec::tcp::Codec;

    pub(crate) struct Proto;

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

    #[cfg(test)]
    mod tests {
        use super::Proto;
        use codec::tcp::Codec;
        use codec::common::CodecType;
        use super::super::dummy_io::DummyIo;

        #[test]
        fn bind_transport() {
            use tokio_proto::pipeline::ClientProto;
            let proto = Proto;
            let io = DummyIo;
            let (_, codec) = proto.bind_transport(io).unwrap().into_parts_and_codec();
            assert_eq!(codec.codec_type, CodecType::Client);
            assert_eq!(codec, Codec::client());
        }
    }
}

#[cfg(feature = "rtu")]
pub mod rtu {

    use frame::RtuAdu;
    use tokio_io::{AsyncRead, AsyncWrite};
    use std::io::Error;
    use tokio_io::codec::Framed;
    use tokio_proto::pipeline::{ClientProto, ServerProto};
    use codec::rtu::Codec;

    pub(crate) struct Proto;

    impl<T: AsyncRead + AsyncWrite + 'static> ClientProto<T> for Proto {
        type Request = RtuAdu;
        type Response = RtuAdu;
        type Transport = Framed<T, Codec>;
        type BindTransport = Result<Self::Transport, Error>;

        fn bind_transport(&self, io: T) -> Self::BindTransport {
            Ok(io.framed(Codec::client()))
        }
    }

    impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for Proto {
        type Request = RtuAdu;
        type Response = RtuAdu;
        type Transport = Framed<T, Codec>;
        type BindTransport = Result<Self::Transport, Error>;

        fn bind_transport(&self, io: T) -> Self::BindTransport {
            Ok(io.framed(Codec::server()))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::Proto;
        use codec::rtu::Codec;
        use super::super::dummy_io::DummyIo;

        #[test]
        fn bind_transport() {
            use tokio_proto::pipeline::ClientProto;
            let proto = Proto;
            let io = DummyIo;
            let (_, codec) = proto.bind_transport(io).unwrap().into_parts_and_codec();
            assert_eq!(codec, Codec::client());
        }
    }
}

#[cfg(test)]
mod dummy_io {
    use std::io::Error;
    use std::io::{Read, Write};
    use tokio_io::{AsyncRead, AsyncWrite};
    use futures::Async;

    pub struct DummyIo;

    impl Read for DummyIo {
        fn read(&mut self, _: &mut [u8]) -> Result<usize, Error> {
            unimplemented!();
        }
    }

    impl Write for DummyIo {
        fn write(&mut self, _: &[u8]) -> Result<usize, Error> {
            unimplemented!();
        }
        fn flush(&mut self) -> Result<(), Error> {
            unimplemented!();
        }
    }

    impl AsyncRead for DummyIo {}

    impl AsyncWrite for DummyIo {
        fn shutdown(&mut self) -> Result<Async<()>, Error> {
            unimplemented!();
        }
    }
}
