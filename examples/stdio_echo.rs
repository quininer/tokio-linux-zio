use std::io;
use std::net::SocketAddr;
use tokio::net::{ TcpListener, TcpStream };
use tokio::io::unix::AsyncFd;
use tokio_linux_zio as zio;


#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = TcpListener::bind(&addr).await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let (mut r, mut w) = stream.split();
                tokio::io::copy(&mut r, &mut w).await.unwrap();
            });
        }
    });

    let (pr, pw) = zio::pipe().unwrap();
    let (pr2, pw2) = zio::pipe().unwrap();

    let stdin = io::stdin();
    let stdout = io::stdout();
    zio::set_nonblocking(&stdin, true).unwrap();
    zio::set_nonblocking(&stdout, true).unwrap();
    let stdin = AsyncFd::new(stdin).unwrap();
    let stdout = AsyncFd::new(stdout).unwrap();

    let stream = TcpStream::connect(&addr).await.unwrap();
    let stream = stream.into_std().unwrap();
    let stream = AsyncFd::new(stream).unwrap();

    tokio::select!{
        ret = zio::splice(&stdin, pw.as_ref(), None) => ret.unwrap(),
        ret = zio::splice(pr.as_ref(), &stream, None) => ret.unwrap(),
        ret = zio::splice(&stream, pw2.as_ref(), None) => ret.unwrap(),
        ret = zio::splice(pr2.as_ref(), &stdout, None) => ret.unwrap()
    };
}
