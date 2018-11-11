mod common;

use std::net::ToSocketAddrs;
use bytes::Bytes;
use tokio::prelude::*;
use tokio::net::TcpStream;
use tokio::runtime::current_thread;
use tokio::io as aio;
use tokio_linux_zio as zio;
use crate::common::run_server;


#[test]
fn test_tcpsink() {
    let addr = run_server();

    let done = TcpStream::connect(&addr)
        .and_then(zio::TcpSink::new)
        .and_then(|sink| {
            stream::iter_ok(vec![&[10u8][..], b"hello", b"world"])
                .map(Bytes::from)
                .forward(sink)
        });

    current_thread::block_on_all(done).unwrap();
}

#[test]
fn test_tcpsink_badssl() {
    let hostname = "http.badssl.com";
    let addr = (hostname, 80)
        .to_socket_addrs().unwrap()
        .next().unwrap();

    let text = format!("\
        GET / HTTP/1.0\r\n\
        Host: {}\r\n\
        Connection: close\r\n\
        \r\n\
    ", hostname);

    let done = TcpStream::connect(&addr)
        .and_then(zio::TcpSink::new)
        .and_then(|sink| sink.send(Bytes::from(text)))
        .and_then(|sink| {
            let (io, _) = sink.into_inner();
            aio::read_to_end(io, Vec::new())
        })
        .map(|(_, buf)| buf);

    let buf = current_thread::block_on_all(done).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.find("<title>http.badssl.com</title>").is_some());
}
