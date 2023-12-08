// mod sendfile;
mod splice;
// mod tee;

use std::io;
use std::pin::Pin;
use std::task::{ ready, Context, Poll };
use std::os::unix::io::{ AsRawFd, RawFd, FromRawFd, OwnedFd };
use tokio::io::{ AsyncRead, AsyncWrite, ReadBuf, Interest };
use tokio::io::unix::AsyncFd;
// pub use crate::sendfile::*;
pub use crate::splice::*;
// pub use crate::tee::*;


pub struct PipeRead(AsyncFd<OwnedFd>);
pub struct PipeWrite(AsyncFd<OwnedFd>);

pub fn pipe() -> io::Result<(PipeRead, PipeWrite)> {
    unsafe {
        let mut pipefd = [0; 2];
        match libc::pipe2(pipefd.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) {
            -1 => Err(io::Error::last_os_error()),
            _ => {
                let pr = OwnedFd::from_raw_fd(pipefd[0]);
                let pw = OwnedFd::from_raw_fd(pipefd[0]);
                let pr = AsyncFd::with_interest(pr, Interest::READABLE)?;
                let pw = AsyncFd::with_interest(pw, Interest::WRITABLE)?;
                Ok((PipeRead(pr), PipeWrite(pw)))
            }
        }
    }
}

impl AsyncRead for PipeRead {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>)
        -> Poll<io::Result<()>>
    {
        loop {
            let mut guard = ready!(self.0.poll_read_ready(cx))?;

            let unfilled = buf.initialize_unfilled();
            match guard.try_io(|inner| unsafe {
                match libc::read(
                    inner.get_ref().as_raw_fd(),
                    unfilled.as_mut_ptr().cast(),
                    unfilled.len()
                ) {
                    -1 => Err(io::Error::last_os_error()),
                    n => Ok(n as usize)
                }
            }) {
                Ok(Ok(len)) => {
                    buf.advance(len);
                    return Poll::Ready(Ok(()));
                },
                Ok(Err(err)) => return Poll::Ready(Err(err)),
                Err(_would_block) => continue,
            }
        }
    }
}

impl AsyncWrite for PipeWrite {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8])
        -> Poll<io::Result<usize>>
    {
        loop {
            let mut guard = ready!(self.0.poll_write_ready(cx))?;

            match guard.try_io(|inner| unsafe {
                match libc::write(
                    inner.get_ref().as_raw_fd(),
                    buf.as_ptr().cast(),
                    buf.len()
                ) {
                    -1 => Err(io::Error::last_os_error()),
                    n => Ok(n as usize)
                }
            }) {
                Ok(result) => return Poll::Ready(result),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>)
        -> Poll<io::Result<()>>
    {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>)
        -> Poll<io::Result<()>>
    {
        Poll::Ready(Ok(()))
    }
}

impl AsRawFd for PipeRead {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl AsRawFd for PipeWrite {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl AsRef<AsyncFd<OwnedFd>> for PipeRead {
    fn as_ref(&self) -> &AsyncFd<OwnedFd> {
        &self.0
    }
}

impl AsRef<AsyncFd<OwnedFd>> for PipeWrite {
    fn as_ref(&self) -> &AsyncFd<OwnedFd> {
        &self.0
    }
}
