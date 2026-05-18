use std::convert::TryFrom;
use std::io::{Error as IoError, Result as IoResult};
use std::os::fd::{FromRawFd as _, RawFd};
use std::net::{
    SocketAddr, TcpListener as StdTcpListener, TcpStream as StdTcpStream,
};

use async_stream::stream;
use tokio::net::{TcpListener, TcpStream};

use super::{Listener, Socket};

impl Listener {
    pub fn bind_tcp(addr: impl AsRef<str>) -> IoResult<Self> {
        let addr = parse_tcp_addr(addr)?;
        let listener = StdTcpListener::bind(addr)?;
        Self::try_from(listener)
    }

    /// # Safety
    /// The file descriptor must represent a tcp listener.
    pub unsafe fn from_raw_tcp_listener_fd(fd: std::os::fd::RawFd) -> IoResult<Self> {
        let listener = unsafe { StdTcpListener::from_raw_fd(fd) };
        Self::try_from(listener)
    }
}

impl Socket {
    pub async fn connect_tcp(addr: impl AsRef<str>) -> IoResult<Self> {
        let addr = parse_tcp_addr(addr)?;
        let socket = StdTcpStream::connect(addr)?;
        Self::try_from(socket)
    }

    /// # Safety
    /// The file descriptor must represent a tcp socket.
    pub unsafe fn from_raw_tcp_socket_fd(fd: RawFd) -> IoResult<Self> {
        let socket = unsafe { StdTcpStream::from_raw_fd(fd) };
        Self::try_from(socket)
    }
}

/// Mirror the sync `common::set_socket_opts` behaviour for the
/// async TCP path: honour `TTRPC_TCP_NODELAY_ENABLED`. Upstream
/// only reads the env var in the sync transport, so every async
/// call pays Nagle's delay (~40 ms/call on localhost).
fn tcp_nodelay_enabled() -> bool {
    match std::env::var("TTRPC_TCP_NODELAY_ENABLED") {
        Ok(v) => v == "1" || v.eq_ignore_ascii_case("true"),
        Err(_) => false,
    }
}

impl From<TcpListener> for Listener {
    fn from(listener: TcpListener) -> Self {
        let nodelay = tcp_nodelay_enabled();
        Self::new(stream! {
            loop {
                yield listener.accept().await.map(|(socket, _)| {
                    if nodelay {
                        let _ = socket.set_nodelay(true);
                    }
                    socket
                });
            }
        })
    }
}

impl TryFrom<StdTcpListener> for Listener {
    type Error = IoError;
    fn try_from(listener: StdTcpListener) -> IoResult<Self> {
        listener.set_nonblocking(true)?;
        Ok(Self::from(TcpListener::from_std(listener)?))
    }
}

impl From<TcpStream> for Socket {
    fn from(socket: TcpStream) -> Self {
        if tcp_nodelay_enabled() {
            let _ = socket.set_nodelay(true);
        }
        Self::new(socket)
    }
}

impl TryFrom<StdTcpStream> for Socket {
    type Error = IoError;
    fn try_from(socket: StdTcpStream) -> IoResult<Self> {
        socket.set_nonblocking(true)?;
        Ok(Self::from(TcpStream::from_std(socket)?))
    }
}

fn parse_tcp_addr(addr: impl AsRef<str>) -> IoResult<SocketAddr> {
    let addr = addr.as_ref();

    addr.parse::<SocketAddr>()
        .map_err(|e| io_other!("Failed to parse TCP address '{}': {}", addr, e))
}
