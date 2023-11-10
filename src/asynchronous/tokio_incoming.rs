// SPDX-License-Identifier: Apache-2.0

use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{ready, Stream};
use tokio::net::{UnixListener, UnixStream, TcpListener, TcpStream};

/// Stream of listeners
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct TokioIncoming<T> {
    inner: T,
}

impl<T> TokioIncoming<T> {
    pub fn new(listener: T) -> Self {
        Self { inner: listener }
    }
}

impl Stream for TokioIncoming<UnixListener> {
    type Item = io::Result<UnixStream>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let (socket, _) = ready!(self.inner.poll_accept(cx))?;
        Poll::Ready(Some(Ok(socket)))
    }
}

impl Stream for TokioIncoming<TcpListener> {
    type Item = io::Result<TcpStream>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let (socket, _) = ready!(self.inner.poll_accept(cx))?;
        Poll::Ready(Some(Ok(socket)))
    }
}

impl<T> AsRawFd for TokioIncoming<T>
where
    T: AsRawFd
{
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}
