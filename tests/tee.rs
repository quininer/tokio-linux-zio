use tokio::io::{ AsyncReadExt, AsyncWriteExt };
use tokio_linux_zio as zio;


#[tokio::test]
async fn test_tee() {
    let (mut pr1, mut pw1) = zio::pipe().unwrap();
    let (mut pr2, pw2) = zio::pipe().unwrap();

    let input = b"hello world!";

    pw1.write_all(input).await.unwrap();
    drop(pw1);

    let len = zio::tee(pr1.as_ref(), pw2.as_ref(), usize::MAX).await.unwrap();
    drop(pw2);

    let mut output = vec![0; len];
    pr1.read_exact(&mut output).await.unwrap();

    assert_eq!(output.len(), input.len());

    let mut output2 = Vec::new();
    pr2.read_to_end(&mut output2).await.unwrap();

    assert_eq!(output, input);
    assert_eq!(output2, input);
}
