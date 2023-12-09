mod common;

use tokio::net::TcpStream;
use tokio::io::unix::AsyncFd;
use tokio::io::{ AsyncReadExt, AsyncWriteExt };
use tokio_linux_zio as zio;
use crate::common::get_server;


#[tokio::test]
async fn test_socket_splice() {
    let addr = get_server().await;

    let expected = "hello world";

    let (mut pr, pw) = zio::pipe().unwrap();

    let mut stream = TcpStream::connect(&addr).await.unwrap();
    stream.write_all(expected.as_bytes()).await.unwrap();
    stream.shutdown().await.unwrap();

    let stream = stream.into_std().unwrap();
    let stream = AsyncFd::new(stream).unwrap();

    zio::splice(&stream, pw.as_ref(), None).await.unwrap();
    drop(pw);

    let mut buf = String::new();
    pr.read_to_string(&mut buf).await.unwrap();

    assert_eq!(buf, expected);
}
