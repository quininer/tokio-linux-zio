use std::{ io, ptr, future };
use std::task::Poll;
use std::os::unix::io::{ AsRawFd, RawFd };
use tokio::io::unix::AsyncFd;


pub async fn splice<R, W>(
    reader: &AsyncFd<R>,
    writer: &AsyncFd<W>,
    len: Option<usize>
)
    -> io::Result<usize>
where
    R: AsRawFd,
    W: AsRawFd
{
    let mut count = 0;

    while len.is_none() || len > Some(count) {
        let min_len = len
            .map(|len| len - count)
            .unwrap_or(libc::PIPE_BUF);

        let eof = future::poll_fn(|cx| loop {
            let reader_poll = reader.poll_read_ready(cx)?;
            let writer_poll = writer.poll_write_ready(cx)?;

            let (mut reader, mut writer) = match (reader_poll, writer_poll) {
                (Poll::Ready(reader), Poll::Ready(writer)) => (reader, writer),
                _ => return Poll::Pending
            };

            return match splice_imp(
                reader.get_ref().as_raw_fd(),
                writer.get_ref().as_raw_fd(),
                min_len
            ) {
                Ok(0) => Poll::Ready(Ok(true)),
                Ok(n) => {
                    count += n;
                    Poll::Ready(Ok(false))
                },
                Err(ref err)
                    if err.kind() == io::ErrorKind::WouldBlock => {
                        // register again
                        reader.clear_ready();
                        writer.clear_ready();
                        continue
                    },
                Err(err) => Poll::Ready(Err(err))
            };
        }).await?;

        if eof {
            break
        }
    }

    Ok(count)
}

fn splice_imp(reader: RawFd, writer: RawFd, len: usize) -> io::Result<usize> {
    unsafe {
        match libc::splice(
            reader,
            ptr::null_mut(),
            writer,
            ptr::null_mut(),
            len,
            libc::SPLICE_F_NONBLOCK
        ) {
            -1 => Err(io::Error::last_os_error()),
            n => Ok(n as usize)
        }
    }
}
