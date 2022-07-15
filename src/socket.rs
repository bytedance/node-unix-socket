use libc;
use napi::{Result};
use crate::util::{get_err};

pub(crate) fn close(fd: i32) -> Result<()> {
  let ret = unsafe { libc::close(fd) };

  if ret != 0 {
    if ret != libc::EINTR && ret != libc::EINPROGRESS {
      return Err(get_err());
    }
  }

  Ok(())
}
