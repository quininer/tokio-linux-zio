use std::io::Write;
use nix::unistd;
use tokio::prelude::*;
use tokio::runtime::current_thread;
use tokio_linux_zio as zio;


#[test]
fn test_tee() {
    let (pr1, mut pw1) = zio::pipe().unwrap();
    let (mut pr2, pw2) = zio::pipe().unwrap();

    let input = b"hello world!";

    pw1.write_all(input).unwrap();
    drop(pw1);

    let done = zio::tee(pr1, pw2)
        .map(|(ifd, _, len)| {
            let mut tmp = vec![0; len];
            unistd::read(ifd, &mut tmp).unwrap();
            tmp
        })
        .concat2();

    let output = current_thread::block_on_all(done).unwrap();

    assert_eq!(output.len(), input.len());

    let mut output2 = Vec::new();
    pr2.read_to_end(&mut output2).unwrap();

    assert_eq!(output, input);
    assert_eq!(output2, input);
}
