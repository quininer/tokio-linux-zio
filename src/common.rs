use std::io;

pub fn cvt(e: nix::Error) -> io::Error {
    match e {
        nix::Error::Sys(errno) => io::Error::from(errno),
        err => io::Error::new(io::ErrorKind::Other, err)
    }
}
