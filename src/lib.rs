mod common;
mod sendfile;
mod splice;
mod tee;
mod tcpsink;

use std::{ io, slice };
use std::marker::PhantomData;
use std::os::unix::io::{ AsRawFd, RawFd };
use nix::unistd;
use nix::sys::uio;
use nix::fcntl::{ fcntl, vmsplice, FcntlArg, OFlag, SpliceFFlags };
use tokio::prelude::*;
use tokio::io::{ AsyncRead, AsyncWrite };
use bytes::Buf;
use iovec::IoVec;
use crate::common::cvt;
pub use crate::sendfile::*;
pub use crate::splice::*;
pub use crate::tee::*;
pub use crate::tcpsink::*;


#[derive(Debug)]
pub enum R {}

#[derive(Debug)]
pub enum W {}

#[derive(Debug)]
pub struct Pipe<T>(pub RawFd, PhantomData<T>);

pub fn pipe() -> io::Result<(Pipe<R>, Pipe<W>)> {
    let (pr, pw) = unistd::pipe().map_err(cvt)?;
    Ok((Pipe(pr, PhantomData), Pipe(pw, PhantomData)))
}

impl<T> From<RawFd> for Pipe<T> {
    fn from(fd: RawFd) -> Pipe<T> {
        Pipe(fd, PhantomData)
    }
}

impl<T> Pipe<T> {
    pub fn set_nonblocking(&self, flag: bool) -> io::Result<()> {
        let mut oflag = fcntl(self.0, FcntlArg::F_GETFL)
            .map(OFlag::from_bits_truncate)
            .map_err(cvt)?;

        if flag {
            oflag.insert(OFlag::O_NONBLOCK);
        } else {
            oflag.remove(OFlag::O_NONBLOCK);
        }

        fcntl(self.0, FcntlArg::F_SETFL(oflag))
            .map(drop)
            .map_err(cvt)
    }
}

macro_rules! try_async {
    ( $e:expr ) => {
        match $e {
            Ok(n) => Ok(Async::Ready(n)),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock =>
                Ok(Async::NotReady),
            Err(e) => Err(e)
        }
    }
}

impl io::Read for Pipe<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unistd::read(self.0, buf).map_err(cvt)
    }
}

impl io::Write for Pipe<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unistd::write(self.0, buf).map_err(cvt)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsyncRead for Pipe<R> {
    unsafe fn prepare_uninitialized_buffer(&self, _: &mut [u8]) -> bool {
        false
    }
}

impl AsyncWrite for Pipe<W> {
    fn write_buf<B: Buf>(&mut self, buf: &mut B) -> Poll<usize, io::Error> {
        static DUMMY: &[u8] = &[0];
        let iovec = <&IoVec>::from(DUMMY);
        let mut bufs = [iovec; 64];
        let n = buf.bytes_vec(&mut bufs);
        let bufs = unsafe {
            slice::from_raw_parts(
                bufs[..n].as_ptr() as *const uio::IoVec<&[u8]>,
                n
            )
        };

        try_async!(vmsplice(self.0, bufs, SpliceFFlags::SPLICE_F_NONBLOCK)
            .map_err(cvt))
    }

    fn shutdown(&mut self) -> Poll<(), io::Error> {
        try_async!(unistd::close(self.0).map_err(cvt))
    }
}

impl<T> AsRawFd for Pipe<T> {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

impl<T> Drop for Pipe<T> {
    fn drop(&mut self) {
        let _ = unistd::close(self.0);
    }
}
