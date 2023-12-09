use std::{ io, future };
use std::os::fd::AsRawFd;
use std::net::{ SocketAddr, Shutdown };
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

    let stdin = io::stdin();
    let stdout = io::stdout();
    zio::set_nonblocking(&stdin, true).unwrap();
    zio::set_nonblocking(&stdout, true).unwrap();
    let stdin = AsyncFd::new(stdin).unwrap();
    let stdout = AsyncFd::new(stdout).unwrap();

    let stream = TcpStream::connect(&addr).await.unwrap();
    let stream = stream.into_std().unwrap();
    let stream = AsyncFd::new(stream).unwrap();

    let (pr, pw) = zio::pipe().unwrap();
    let (pr2, pw2) = zio::pipe().unwrap();
    let mut pw = Some(pw);
    let mut pw2 = Some(pw2);
    let mut sw = Some(&stream);

    loop {
        tokio::select!{
            ret = maybe_splice(&stdin, pw.as_ref().map(AsRef::as_ref)) => {
                ret.unwrap();
                pw.take();
            },
            ret = maybe_splice(pr.as_ref(), sw) => {
                ret.unwrap();
                sw.take();
                stream.get_ref().shutdown(Shutdown::Write).unwrap();
            },
            ret = maybe_splice(&stream, pw2.as_ref().map(AsRef::as_ref)) => {
                ret.unwrap();
                pw2.take();
            },
            ret = zio::splice(pr2.as_ref(), &stdout, None) => {
                ret.unwrap();
                break
            },
        };
    }
}

async fn maybe_splice<R, W>(reader: &AsyncFd<R>, writer: Option<&AsyncFd<W>>)
    -> io::Result<usize>
where
    R: AsRawFd,
    W: AsRawFd
{
    if let Some(writer) = writer {
        zio::splice(reader, writer, None).await
    } else {
        future::pending::<()>().await;
        Ok(0)
    }
}
