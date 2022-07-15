use std::mem;
use std::{str::FromStr};

use libc::{sockaddr_un};
use napi::{Result, Env};
use uv_sys::sys;
use crate::util::{get_err, error};

pub (crate) fn get_loop(env: &Env) -> Result<*mut sys::uv_loop_t> {
  Ok(env.get_uv_event_loop()? as *mut _ as *mut sys::uv_loop_t)
}

pub(crate) fn close(fd: i32) -> Result<()> {
  let ret = unsafe { libc::close(fd) };

  if ret != 0 {
    if ret != libc::EINTR && ret != libc::EINPROGRESS {
      return Err(get_err());
    }
  }

  Ok(())
}
pub(crate) unsafe fn sockaddr_from_string(bytes: &str) -> Result<(sockaddr_un, usize)> {
  let mut bytes = String::from_str(bytes).unwrap();
  bytes.push('\0');

  let mut sockaddr = mem::MaybeUninit::<sockaddr_un>::zeroed().assume_init();
  sockaddr.sun_len = 0;
  // looks like `sun_len` is not necessary
  // (*sockaddr).sun_len = bytes.len() as u8;
  sockaddr.sun_family = libc::AF_UNIX as u8;

  let size = mem::size_of_val(&sockaddr.sun_path);

  if bytes.len() > size {
    return Err(error("path to bind is too long".to_string()));
  }
  let path = (&mut sockaddr.sun_path.as_mut_slice()[0..bytes.len()]) as *mut _ as *mut [u8];
  let path = &mut *path;
  path.clone_from_slice(bytes.as_bytes());

  Ok((sockaddr, mem::size_of::<sockaddr_un>()))
}
