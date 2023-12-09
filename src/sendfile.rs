use std::{ fs, io };
use std::ops::{ Bound, RangeBounds };
use std::os::unix::io::{ AsRawFd, RawFd };
use tokio::io::unix::AsyncFd;


pub async fn sendfile<W, R>(writer: &AsyncFd<W>, fd: &fs::File, range: R)
    -> io::Result<usize>
where
    W: AsRawFd,
    R: RangeBounds<usize>
{
    let offset = match range.start_bound() {
        Bound::Excluded(&x) | Bound::Included(&x) => x,
        Bound::Unbounded => 0
    };

    let len = match range.end_bound() {
        Bound::Excluded(&y) => y - offset,
        Bound::Included(&y) => y + 1 - offset,
        Bound::Unbounded => match fd.metadata() {
            Ok(metadata) => metadata.len() as _,
            Err(err) => return Err(err)
        }
    };

    let mut offset = offset as _;
    let mut count = 0;

    while len > count {
        let len = len - count;

        let mut guard = writer.writable().await?;

        match guard.try_io(|inner| sendfile_imp(
            inner.get_ref().as_raw_fd(),
            fd.as_raw_fd(),
            &mut offset,
            len
        )) {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => count += n,
            Ok(Err(err)) => return Err(err),
            Err(_would_block) => continue
        }
    }

    Ok(count)
}

fn sendfile_imp(writer: RawFd, reader: RawFd, offset: &mut libc::off_t, len: usize)
    -> io::Result<usize>
{
    unsafe {
        match libc::sendfile(
            writer,
            reader,
            offset,
            len
        ) {
            -1 => Err(io::Error::last_os_error()),
            n => Ok(n as usize)
        }
    }
}
