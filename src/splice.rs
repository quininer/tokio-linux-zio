use std::{ cmp, mem, io };
use std::os::unix::io::AsRawFd;
use tokio::prelude::*;
use nix::libc::{ PIPE_BUF, loff_t };
use nix::fcntl::{ SpliceFFlags, splice as nix_splice };
use crate::common::cvt;


#[derive(Debug)]
pub struct Splice<R, W>(State<R, W>);

#[derive(Debug)]
enum State<R, W> {
    Writing {
        reader: R,
        writer: W,
        off_in: Option<loff_t>,
        off_out: Option<loff_t>,
        buff_len: usize,
        len: Option<usize>,
        flags: SpliceFFlags,
        sum: usize
    },
    End
}

pub fn splice<R, W>(reader: R, writer: W, len: Option<usize>)
    -> Splice<R, W>
where
    R: AsRawFd + io::Read,
    W: AsRawFd + io::Write
{
    Splice(State::Writing {
        reader, writer, len,
        off_in: None, off_out: None,
        buff_len: PIPE_BUF,
        flags: SpliceFFlags::SPLICE_F_NONBLOCK,
        sum: 0
    })
}

pub fn full_splice<R: AsRawFd, W: AsRawFd>(
    reader: R,
    off_in: Option<loff_t>,
    writer: W,
    off_out: Option<loff_t>,
    buff_len: usize,
    len: Option<usize>,
    flags: SpliceFFlags
) -> Splice<R, W> {
    Splice(State::Writing {
        reader, writer, off_in, off_out,
        buff_len, len, flags,
        sum: 0
    })
}

impl<R: AsRawFd, W: AsRawFd> Future for Splice<R, W> {
    type Item = (R, W, usize);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0 {
            State::Writing {
                ref reader, ref mut off_in,
                ref writer, ref mut off_out,
                buff_len, ref mut len, flags,
                ref mut sum
            } => while len != &Some(0) {
                let len2 = cmp::min(buff_len, len.unwrap_or(buff_len));
                match nix_splice(
                    reader.as_raw_fd(), off_in.as_mut(),
                    writer.as_raw_fd(), off_out.as_mut(),
                    len2, flags
                ).map_err(cvt) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Some(len) = len {
                            *len -= n;
                        }
                        *sum += n;
                    },
                    Err(ref err) if io::ErrorKind::WouldBlock == err.kind()
                        => return Ok(Async::NotReady),
                    Err(err) => return Err(err)
                }
            },
            State::End => panic!()
        }

        match mem::replace(&mut self.0, State::End) {
            State::Writing { reader, writer, sum, .. }
                => Ok((reader, writer, sum).into()),
            _ => panic!()
        }
    }
}
