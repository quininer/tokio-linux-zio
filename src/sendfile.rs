use std::{ fs, io, mem };
use std::ops::{ RangeBounds, Bound };
use std::os::unix::io::AsRawFd;
use tokio::prelude::*;
use nix::libc::{ off_t, size_t, };
use nix::sys::sendfile::sendfile as nix_sendfile;
use crate::common::io_err;


#[derive(Debug)]
pub struct SendFile<IO>(io::Result<State<IO>>);

#[derive(Debug)]
enum State<IO> {
    Writing {
        io: IO,
        fd: fs::File,
        offset: off_t,
        count: size_t
    },
    End
}

pub fn sendfile<IO, R>(io: IO, fd: fs::File, range: R)
    -> SendFile<IO>
where
    IO: AsRawFd,
    R: RangeBounds<usize>
{
    let offset = match range.start_bound() {
        Bound::Excluded(&x) | Bound::Included(&x) => x,
        Bound::Unbounded => 0
    };

    let count = match range.end_bound() {
        Bound::Excluded(&y) => y - offset,
        Bound::Included(&y) => y + 1 - offset,
        Bound::Unbounded => match fd.metadata() {
            Ok(metadata) => metadata.len() as _,
            Err(err) => return SendFile(Err(err))
        }
    };

    let offset = offset as _;

    SendFile(Ok(State::Writing { io, fd, offset, count }))
}

impl<IO: AsRawFd> Future for SendFile<IO> {
    type Item = (IO, fs::File);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if self.0.is_err() {
            mem::replace(&mut self.0, Ok(State::End))?;
        }

        match self.0.as_mut() {
            Ok(State::Writing { io, fd, ref mut offset, ref mut count }) => while *count > 0 {
                match nix_sendfile(io.as_raw_fd(), fd.as_raw_fd(), Some(offset), *count)
                    .map_err(io_err)
                {
                    Ok(n) => *count -= n,
                    Err(ref err) if io::ErrorKind::WouldBlock == err.kind()
                        => return Ok(Async::NotReady),
                    Err(err) => return Err(err)
                }
            },
            _ => panic!()
        }

        match mem::replace(&mut self.0, Ok(State::End)) {
            Ok(State::Writing { io, fd, .. }) => Ok((io, fd).into()),
            _ => panic!()
        }
    }
}
