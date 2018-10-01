use std::os::unix::io::AsRawFd;
use nix::unistd;
use tokio::prelude::*;
use tokio::runtime::current_thread;
use tokio_linux_zio as zio;


#[test]
fn test_tee() {
    let (pr1, pw1) = zio::pipe().unwrap();
    let (pr2, pw2) = zio::pipe().unwrap();

    let input = b"hello world!";

    unistd::write(pw1.as_raw_fd(), input).unwrap();
    unistd::close(pw1.as_raw_fd()).unwrap();

    let done = zio::tee(pr1, pw2)
        .map(|(i, _, l)| {
            let mut tmp = vec![0; l];
            unistd::read(i.as_raw_fd(), &mut tmp).unwrap();
            tmp
        })
        .concat2();

    let output = current_thread::block_on_all(done).unwrap();

    assert_eq!(output.len(), input.len());

    let mut output2 = vec![0; output.len()];
    unistd::read(pr2.as_raw_fd(), &mut output2).unwrap();

    assert_eq!(output, input);
    assert_eq!(output2, input);
}
