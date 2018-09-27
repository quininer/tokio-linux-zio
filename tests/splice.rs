use std::thread;
use std::sync::mpsc::channel;
use std::net::SocketAddr;
use std::os::unix::io::{ AsRawFd, RawFd };
use lazy_static::lazy_static;
use tokio::prelude::*;
use tokio::net::{ TcpListener, TcpStream };
use tokio::io as aio;
use tokio::runtime::current_thread;
use nix::unistd;
use tokio_linux_io as lio;


lazy_static! {
    static ref TEST_SERVER: SocketAddr = {
        let (send, recv) = channel();

        thread::spawn(move || {
            let addr = SocketAddr::from(([127, 0, 0, 1], 0));
            let listener = TcpListener::bind(&addr).unwrap();

            send.send(listener.local_addr().unwrap()).unwrap();

            let done = listener.incoming()
                .for_each(|stream| {
                    let done = aio::read_exact(stream, [0; 1])
                        .and_then(|(stream, buf)| aio::read_exact(stream, vec![0; buf[0] as usize]))
                        .and_then(|(stream, buf)| aio::write_all(stream, buf))
                        .map(drop)
                        .map_err(|err| eprintln!("{:?}", err));
                    tokio::spawn(done);
                    Ok(())
                });

            current_thread::block_on_all(done).unwrap();
        });

        recv.recv().unwrap()
    };
}

fn run_server() -> SocketAddr {
    *TEST_SERVER
}

struct Pipe(RawFd);

impl AsRawFd for Pipe {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

#[test]
fn test_socket_splice() {
    let addr = run_server();

    let (pr, pw) = unistd::pipe().unwrap();

    let done = TcpStream::connect(&addr)
        .and_then(|stream| aio::write_all(stream, b"\x0cHello world!"))
        .and_then(|(stream, _)| lio::splice(stream, Pipe(pw)))
        .map(|(.., len)| len);

    let len = current_thread::block_on_all(done).unwrap();

    let mut buf = vec![0; len];
    unistd::read(pr, &mut buf).unwrap();
    assert_eq!(buf, b"Hello world!");
}
