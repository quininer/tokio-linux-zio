use std::io::Cursor;
use tokio::prelude::*;
use tokio::io as aio;
use tokio::runtime::current_thread;
use tokio_linux_zio as zio;


#[test]
fn test_write_buf() {
    let (pr, mut pw) = zio::pipe().unwrap();
    pr.set_nonblocking(true).unwrap();
    pw.set_nonblocking(true).unwrap();

    let input = b"hello world!";

    let done = future::poll_fn(|| {
            let mut input = Cursor::new(input);
            pw.write_buf(&mut input)
        })
        .and_then(move |n| aio::read_exact(pr, vec![0; n]));

    let (_, buf) = current_thread::block_on_all(done).unwrap();

    assert_eq!(buf, input);
}
