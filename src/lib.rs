mod common;
mod sendfile;
mod splice;
mod tee;

use std::io;
use std::os::unix::io::{ AsRawFd, RawFd };
use nix::unistd::pipe;
use crate::common::io_err;
pub use crate::sendfile::*;
pub use crate::splice::*;
pub use crate::tee::*;


pub struct Pipe(RawFd);

impl Pipe {
    pub fn new() -> io::Result<(Pipe, Pipe)> {
        let (pr, pw) = pipe().map_err(io_err)?;
        Ok((Pipe(pr), Pipe(pw)))
    }
}

impl AsRawFd for Pipe {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}
