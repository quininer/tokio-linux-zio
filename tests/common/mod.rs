use std::net::SocketAddr;
use tokio::sync::{ oneshot, OnceCell };
use tokio::net::TcpListener;


static TEST_SERVER: OnceCell<SocketAddr> = OnceCell::const_new();

async fn init_server() -> SocketAddr {
    let (send, recv) = oneshot::channel();

    tokio::spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(&addr).await.unwrap();

        send.send(listener.local_addr().unwrap()).unwrap();

        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let (mut r, mut w) = stream.split();
                tokio::io::copy(&mut r, &mut w).await.unwrap();
            });
        }
    });

    recv.await.unwrap()
}

pub async fn get_server() -> SocketAddr {
    *TEST_SERVER.get_or_init(init_server).await
}
