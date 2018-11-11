use std::io;
use std::os::unix::io::{ AsRawFd, RawFd };
use nix::fcntl::{ SpliceFFlags, tee as nix_tee };
use tokio::prelude::*;
use crate::common::cvt;
use crate::{ Pipe, R, W };


#[derive(Debug)]
pub struct Tee {
    input: Pipe<R>,
    output: Pipe<W>,
    len: usize,
    flags: SpliceFFlags
}

pub fn tee(input: Pipe<R>, output: Pipe<W>) -> Tee {
    Tee {
        input: input,
        output: output,
        len: usize::max_value(),
        flags: SpliceFFlags::SPLICE_F_NONBLOCK,
    }
}

pub fn full_tee(
    input: Pipe<R>,
    output: Pipe<W>,
    len: usize,
    flags: SpliceFFlags
) -> Tee {
    Tee { input, output, len, flags }
}

impl Stream for Tee {
    type Item = (RawFd, RawFd, usize);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let Tee { ref input, ref output, len, flags } = *self;
        let ifd = input.as_raw_fd();
        let ofd = output.as_raw_fd();

        match nix_tee(ifd, ofd, len, flags).map_err(cvt) {
            Ok(0) => Ok(Async::Ready(None)),
            Ok(n) => Ok(Async::Ready(Some((ifd, ofd, n)))),
            Err(ref err) if io::ErrorKind::WouldBlock == err.kind()
                => Ok(Async::NotReady),
            Err(err) => Err(err)
        }
    }
}
