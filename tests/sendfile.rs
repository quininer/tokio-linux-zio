mod common;

use std::fs;
use std::io::Read;
use tokio::prelude::*;
use tokio::net::TcpStream;
use tokio::io as aio;
use tokio::runtime::current_thread;
use tokio_linux_zio as zio;
use crate::common::run_server;


#[test]
fn test_sendfile() {
    let addr = run_server();

    let fd = fs::File::open("Cargo.toml").unwrap();

    let done = TcpStream::connect(&addr)
        .and_then(|stream| aio::write_all(stream, [32]))
        .and_then(move |(stream, _)| zio::sendfile(stream, fd, ..32))
        .and_then(|(stream, fd, len)| aio::read_exact(stream, vec![0; len])
            .map(move |(stream, buf)| (stream, fd, buf))
        );

    let (_, mut fd, buf) = current_thread::block_on_all(done).unwrap();

    let mut buf2 = vec![0; buf.len()];
    fd.read_exact(&mut buf2).unwrap();

    assert_ne!(buf[0], 0);
    assert_eq!(buf.len(), 32);
    assert_eq!(buf, buf2);
}
