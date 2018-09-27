use std::{ cmp, mem, io };
use std::os::unix::io::AsRawFd;
use nix::libc::PIPE_BUF;
use nix::fcntl::{ SpliceFFlags, tee as nix_tee };
use tokio::prelude::*;
use crate::common::io_err;


pub struct Tee<I, O>(Option<State<I, O>>);

struct State<I, O> {
    input: I,
    output: O,
    buff_len: usize,
    len: Option<usize>,
    flags: SpliceFFlags,
    sum: usize
}

pub fn tee<I: AsRawFd, O: AsRawFd>(input: I, output: O) -> Tee<I, O> {
    Tee(Some(State {
        input, output,
        buff_len: PIPE_BUF, len: None,
        flags: SpliceFFlags::SPLICE_F_NONBLOCK,
        sum: 0
    }))
}

pub fn full_tee<I: AsRawFd, O: AsRawFd>(
    input: I,
    output: O,
    buff_len: usize,
    len: Option<usize>,
    flags: SpliceFFlags
) -> Tee<I, O> {
    Tee(Some(State { input, output, buff_len, len, flags, sum: 0 }))
}

impl<I: AsRawFd, O: AsRawFd> Future for Tee<I, O> {
    type Item = (I, O, usize);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0 {
            Some(State {
                ref input, ref output,
                buff_len, flags,
                ref mut len, ref mut sum
            }) => while len != &Some(0) {
                let len2 = cmp::min(buff_len, len.unwrap_or(buff_len));
                match nix_tee(input.as_raw_fd(), output.as_raw_fd(), len2, flags)
                    .map_err(io_err)
                {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Some(len) = len {
                            *len -= n;
                        }
                        *sum += n
                    },
                    Err(ref err) if io::ErrorKind::WouldBlock == err.kind()
                        => return Ok(Async::NotReady),
                    Err(err) => return Err(err)
                }
            },
            None => panic!()
        }

        match mem::replace(&mut self.0, None) {
            Some(State { input, output, sum, .. })
                => Ok((input, output, sum).into()),
            _ => panic!()
        }
    }
}
