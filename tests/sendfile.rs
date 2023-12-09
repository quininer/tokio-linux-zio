mod common;

use std::fs;
use std::io::Read;
use tokio::net::TcpStream;
use tokio::io::unix::AsyncFd;
use tokio::io::AsyncReadExt;
use tokio_linux_zio as zio;
use crate::common::get_server;


#[tokio::test]
async fn test_sendfile() {
    let addr = get_server().await;

    let mut fd = fs::File::open("Cargo.toml").unwrap();

    let stream = TcpStream::connect(&addr).await.unwrap();
    let stream = stream.into_std().unwrap();
    let stream = AsyncFd::new(stream).unwrap();

    zio::sendfile(&stream, &fd, 1..33).await.unwrap();

    let stream = stream.into_inner();
    let mut stream = TcpStream::from_std(stream).unwrap();
    let mut buf = vec![0; 32];
    stream.read_exact(&mut buf).await.unwrap();

    let mut buf2 = vec![0; 33];
    fd.read_exact(&mut buf2).unwrap();

    assert_ne!(buf[0], 0);
    assert_eq!(buf.len(), 32);
    assert_eq!(buf, &buf2[1..]);
}
