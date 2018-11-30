#[cfg(feature = "tcp")]
pub mod tcp;

#[cfg(feature = "rtu")]
pub mod rtu;

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
