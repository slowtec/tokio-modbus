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
