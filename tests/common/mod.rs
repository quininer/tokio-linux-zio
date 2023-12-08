use std::thread;
use std::sync::mpsc::channel;
use std::net::SocketAddr;
use std::sync::OnceLock;
use tokio::net::TcpListener;
use tokio::runtime::current_thread;


static TEST_SERVER: OnceLock<SocketAddr> = OnceLock::new();

fn init_server() -> SocketAddr {
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
}

pub fn get_server() -> SocketAddr {
    *TEST_SERVER.get_or_init(init_server)
}
