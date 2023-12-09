use std::{ io, future };
use std::task::Poll;
use std::os::unix::io::{ AsRawFd, RawFd };
use tokio::io::unix::AsyncFd;


pub async fn tee<R, W>(
    reader: &AsyncFd<R>,
    writer: &AsyncFd<W>,
    len: usize
)
    -> io::Result<usize>
where
    R: AsRawFd,
    W: AsRawFd
{
    future::poll_fn(|cx| loop {
        let reader_poll = reader.poll_read_ready(cx)?;
        let writer_poll = writer.poll_write_ready(cx)?;

        let (mut reader, mut writer) = match (reader_poll, writer_poll) {
            (Poll::Ready(reader), Poll::Ready(writer)) => (reader, writer),
            _ => return Poll::Pending
        };

        return match tee_imp(
            reader.get_ref().as_raw_fd(),
            writer.get_ref().as_raw_fd(),
            len
        ) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(ref err)
                if err.kind() == io::ErrorKind::WouldBlock => {
                    // register again
                    reader.clear_ready();
                    writer.clear_ready();
                    continue
                },
            Err(err) => Poll::Ready(Err(err))
        };
    }).await
}

fn tee_imp(fd_in: RawFd, fd_out: RawFd, len: usize)
    -> io::Result<usize>
{
    unsafe {
        match libc::tee(fd_in, fd_out, len, libc::SPLICE_F_NONBLOCK) {
            -1 => Err(io::Error::last_os_error()),
            n => Ok(n as usize)
        }
    }
}
