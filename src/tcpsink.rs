use std::{ io, mem };
use std::os::unix::io::AsRawFd;
use bytes::Bytes;
use tokio::prelude::*;
use tokio::net::TcpStream;
use nix::libc;
use nix::sys::socket;
use crate::common::cvt;


const SO_ZEROCOPY: libc::c_int = 60;
const MSG_ZEROCOPY: libc::c_int = 0x4000000;
const SO_EE_ORIGIN_ZEROCOPY: u8 = 5;
const IP_RECVERR: libc::c_int = 11;
const IPV6_RECVERR: libc::c_int = 25;

macro_rules! cmsg {
    ( firsthdr $mhdr:expr ) => {
        $mhdr.msg_control as *mut libc::cmsghdr
    };
    ( data $cmsg:expr ) => {
        $cmsg.add(1) as *mut u8
    };
}

pub struct TcpSink {
    pub io: TcpStream,
    buf: Vec<Bytes>
}

impl TcpSink {
    pub fn new(io: TcpStream) -> io::Result<TcpSink> {
        unsafe {
            let one: libc::c_int = 1;
            if libc::setsockopt(
                io.as_raw_fd(),
                libc::SOL_SOCKET,
                SO_ZEROCOPY,
                &one as *const libc::c_int as *const _,
                mem::size_of::<libc::c_int>() as _
            ) == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(TcpSink { io, buf: Vec::new() })
            }
        }
    }

    pub fn as_ref(&self) -> &[Bytes] {
        &self.buf
    }

    pub fn into_inner(self) -> (TcpStream, Vec<Bytes>) {
        (self.io, self.buf)
    }
}

impl Sink for TcpSink {
    type SinkItem = Bytes;
    type SinkError = io::Error;

    fn start_send(&mut self, mut item: Self::SinkItem)
        -> Result<AsyncSink<Self::SinkItem>, Self::SinkError>
    {
        if item.is_empty() {
            return Ok(AsyncSink::Ready);
        }

        let flag = socket::MsgFlags::from_bits_truncate(MSG_ZEROCOPY);
        match socket::send(self.io.as_raw_fd(), item.as_ref(), flag)
            .map_err(cvt)
        {
            Ok(0) => Ok(AsyncSink::NotReady(item)),
            Ok(n) if item.len() == n => {
                self.buf.push(item);
                Ok(AsyncSink::Ready)
            },
            Ok(n) => {
                let buf = item.split_off(n);
                self.buf.push(buf);
                Ok(AsyncSink::NotReady(item))
            },
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock =>
                return Ok(AsyncSink::NotReady(item)),
            Err(e) => return Err(e)
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        unsafe {
            let mut msg: libc::msghdr = mem::zeroed();
            let mut control = [0; 1024];
            msg.msg_control = control.as_mut_ptr() as *mut _;
            msg.msg_controllen = control.len() as _;

            assert!(control.len() >= mem::size_of::<libc::cmsghdr>());

            while libc::recvmsg(self.io.as_raw_fd(), &mut msg, libc::MSG_ERRQUEUE) == -1 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::WouldBlock {
                    // TODO handle

                    return Ok(Async::NotReady);
                } else {
                    return Err(err);
                }
            }

            // TODO check truncate

            let cmsg = cmsg!(firsthdr &msg);
            if !(((*cmsg).cmsg_level == libc::SOL_IP && (*cmsg).cmsg_type == IP_RECVERR)
                || ((*cmsg).cmsg_level == libc::SOL_IPV6 && (*cmsg).cmsg_type == IPV6_RECVERR))
            {
                return Err(io::Error::from(io::ErrorKind::Other));
            }

            let serr = cmsg!(data cmsg) as *const sock_extended_err;
            if (*serr).ee_errno != 0 || (*serr).ee_origin != SO_EE_ORIGIN_ZEROCOPY {
                return Err(io::Error::from(io::ErrorKind::Other));
            }

            eprintln!("> info: {:?}, data: {:?}", (*serr).ee_info, (*serr).ee_data);

            // TODO

            Ok(Async::Ready(()))
        }
    }
}

#[repr(C)]
struct sock_extended_err {
    ee_errno: u32,
    ee_origin: u8,
    ee_type: u8,
    ee_code: u8,
    ee_pad: u8,
    ee_info: u32,
    ee_data: u32
}
