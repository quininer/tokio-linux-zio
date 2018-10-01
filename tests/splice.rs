mod common;

use tokio::prelude::*;
use tokio::net::TcpStream;
use tokio::io as aio;
use tokio::runtime::current_thread;
use tokio_linux_zio as zio;
use crate::common::run_server;


#[test]
fn test_socket_splice() {
    let addr = run_server();

    let (pr, pw) = zio::pipe().unwrap();
    pr.set_nonblocking(true).unwrap();
    pw.set_nonblocking(true).unwrap();

    let done = TcpStream::connect(&addr)
        .and_then(|stream| aio::write_all(stream, b"\x0cHello world!"))
        .and_then(|(stream, _)| zio::splice(stream, pw, None))
        .and_then(move |(.., len)| aio::read_exact(pr, vec![0; len]));

    let (_, buf) = current_thread::block_on_all(done).unwrap();

    assert_eq!(buf, b"Hello world!");
}
