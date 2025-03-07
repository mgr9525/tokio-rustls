use rustls::{ServerConfig, ServerConnection};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::common::TlsState;
use crate::MidHandshake;
pub struct Acceptor<'a, IO> {
    io: &'a mut IO,
    acceptor: rustls::server::Acceptor,
}

impl<'a, IO: AsyncRead + AsyncWrite + Unpin> Acceptor<'a, IO> {
    pub fn new(io: &'a mut IO) -> Acceptor<'a, IO> {
        Acceptor {
            io,
            acceptor: rustls::server::Acceptor::default(),
        }
    }

    pub fn read_tls(&mut self) -> ReadTls<'_, IO> {
        ReadTls {
            io: &mut self.io,
            acceptor: &mut self.acceptor,
        }
    }

    pub fn accept(&mut self) -> Accepted<'_> {
        Accepted {
            acceptor: &mut self.acceptor,
        }
    }
    pub fn from_srvconn(conn: ServerConnection, stream: IO) -> crate::Accept<IO> {
        crate::Accept(MidHandshake::Handshaking(crate::server::TlsStream {
            session: conn,
            io: stream,
            state: TlsState::Stream,
        }))
    }
    /* pub fn accept(&mut self) -> Poll<io::Result<rustls::server::Accepted>> {
        match self.acceptor.accept() {
            Ok(Some(accepted)) => Poll::Ready(Ok(accepted)),
            Ok(None) => Poll::Pending,
            Err((err, alert)) => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::Other,
                format!("accepted err{}", err),
            ))),
        }
    } */

    // pub(crate) fn read_tls_inner(&mut self, rd: &mut crate::rusttls::StdReader<'_>) -> io::Result<usize> {
}

pub struct Accepted<'a> {
    acceptor: &'a mut rustls::server::Acceptor,
}

impl<'a> Future for Accepted<'a> {
    type Output = std::io::Result<rustls::server::Accepted>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.acceptor.accept() {
            Ok(Some(accepted)) => Poll::Ready(Ok(accepted)),
            Ok(None) => Poll::Pending,
            Err((err, alert)) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("accepted err{}", err),
            ))),
        }
    }
}
pub struct ReadTls<'a, IO> {
    io: &'a mut IO,
    acceptor: &'a mut rustls::server::Acceptor,
}

impl<'a, IO: AsyncRead + AsyncWrite + Unpin> Future for ReadTls<'a, IO> {
    type Output = std::io::Result<usize>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let mut rd = StdReader::new(this.io, cx);
        match this.acceptor.read_tls(&mut rd) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => return Poll::Pending,
            Err(err) => return Poll::Ready(Err(err)),
        }
    }
}

pub struct StdReader<'a, 'b, T> {
    io: &'a mut T,
    cx: &'a mut std::task::Context<'b>,
}

impl<'a, 'b, T: AsyncRead + Unpin> StdReader<'a, 'b, T> {
    pub fn new(io: &'a mut T, cx: &'a mut std::task::Context<'b>) -> Self {
        Self { io, cx }
    }
}

impl<'a, 'b, T: AsyncRead + Unpin> std::io::Read for StdReader<'a, 'b, T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut rbf = tokio::io::ReadBuf::new(buf);
        match std::pin::Pin::new(&mut self.io).poll_read(self.cx, &mut rbf) {
            std::task::Poll::Ready(result) => match result {
                Ok(_) => Ok(rbf.filled().len()),
                Err(err) => Err(err.into()),
            },
            std::task::Poll::Pending => Err(std::io::ErrorKind::WouldBlock.into()),
        }
    }
}
