#[cfg(feature = "tcp")]
pub mod tcp {

    use crate::codec::tcp::Codec;
    use crate::frame::TcpAdu;
    use std::io::Error;
    use tokio_codec::{Decoder, Framed};
    use tokio_io::{AsyncRead, AsyncWrite};
    use tokio_proto::pipeline::{ClientProto, ServerProto};

    pub(crate) struct Proto;

    impl<T: AsyncRead + AsyncWrite + 'static> ClientProto<T> for Proto {
        type Request = TcpAdu;
        type Response = TcpAdu;
        type Transport = Framed<T, Codec>;
        type BindTransport = Result<Self::Transport, Error>;

        fn bind_transport(&self, io: T) -> Self::BindTransport {
            Ok(Codec::client().framed(io))
        }
    }

    impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for Proto {
        type Request = TcpAdu;
        type Response = TcpAdu;
        type Transport = Framed<T, Codec>;
        type BindTransport = Result<Self::Transport, Error>;

        fn bind_transport(&self, io: T) -> Self::BindTransport {
            Ok(Codec::server().framed(io))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::dummy_io::DummyIo;
        use super::Proto;
        use crate::codec::common::CodecType;
        use crate::codec::tcp::Codec;

        #[test]
        fn bind_transport() {
            use tokio_proto::pipeline::ClientProto;
            let proto = Proto;
            let io = DummyIo;
            let parts = proto.bind_transport(io).unwrap().into_parts();
            assert_eq!(parts.codec.codec_type, CodecType::Client);
            assert_eq!(parts.codec, Codec::client());
        }
    }
}

#[cfg(feature = "rtu")]
pub mod rtu {

    use crate::codec::rtu::Codec;
    use crate::frame::RtuAdu;
    use std::io::Error;
    use tokio_codec::{Decoder, Framed};
    use tokio_io::{AsyncRead, AsyncWrite};
    use tokio_proto::pipeline::{ClientProto, ServerProto};

    pub(crate) struct Proto;

    impl<T: AsyncRead + AsyncWrite + 'static> ClientProto<T> for Proto {
        type Request = RtuAdu;
        type Response = RtuAdu;
        type Transport = Framed<T, Codec>;
        type BindTransport = Result<Self::Transport, Error>;

        fn bind_transport(&self, io: T) -> Self::BindTransport {
            Ok(Codec::client().framed(io))
        }
    }

    impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for Proto {
        type Request = RtuAdu;
        type Response = RtuAdu;
        type Transport = Framed<T, Codec>;
        type BindTransport = Result<Self::Transport, Error>;

        fn bind_transport(&self, io: T) -> Self::BindTransport {
            Ok(Codec::server().framed(io))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::dummy_io::DummyIo;
        use super::Proto;
        use crate::codec::rtu::Codec;

        #[test]
        fn bind_transport() {
            use tokio_proto::pipeline::ClientProto;
            let proto = Proto;
            let io = DummyIo;
            let parts = proto.bind_transport(io).unwrap().into_parts();
            assert_eq!(parts.codec, Codec::client());
        }
    }
}

#[cfg(test)]
mod dummy_io {
    use futures::Async;
    use std::io::Error;
    use std::io::{Read, Write};
    use tokio_io::{AsyncRead, AsyncWrite};

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
