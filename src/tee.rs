use std::io;
use std::rc::Rc;
use std::os::unix::io::AsRawFd;
use nix::fcntl::{ SpliceFFlags, tee as nix_tee };
use tokio::prelude::*;
use crate::common::io_err;
use crate::{ Pipe, R, W };


#[derive(Debug)]
pub struct Tee {
    input: Rc<Pipe<R>>,
    output: Rc<Pipe<W>>,
    len: usize,
    flags: SpliceFFlags
}

pub fn tee(input: Pipe<R>, output: Pipe<W>) -> Tee {
    Tee {
        input: Rc::new(input),
        output: Rc::new(output),
        len: usize::max_value(),
        flags: SpliceFFlags::SPLICE_F_NONBLOCK,
    }
}

pub fn full_tee(
    input: Rc<Pipe<R>>,
    output: Rc<Pipe<W>>,
    len: usize,
    flags: SpliceFFlags
) -> Tee {
    Tee { input, output, len, flags }
}

impl Stream for Tee {
    type Item = (Rc<Pipe<R>>, Rc<Pipe<W>>, usize);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let Tee { ref input, ref output, len, flags } = *self;
        match nix_tee(input.as_raw_fd(), output.as_raw_fd(), len, flags)
            .map_err(io_err)
        {
            Ok(0) => Ok(Async::Ready(None)),
            Ok(n) => Ok(Async::Ready(Some((input.clone(), output.clone(), n)))),
            Err(ref err) if io::ErrorKind::WouldBlock == err.kind()
                => Ok(Async::NotReady),
            Err(err) => Err(err)
        }
    }
}
