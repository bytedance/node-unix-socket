use std::ffi::CStr;
use std::intrinsics::transmute;
use std::mem;

use libc::{sockaddr, sockaddr_un};
use napi::{self, Error, Result};
use nix::errno::Errno;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use uv_sys::sys;

pub fn i8_slice_into_u8_slice<'a>(slice: &'a [i8]) -> &'a [u8] {
  unsafe { &*(slice as *const [i8] as *const [u8]) }
}

pub fn addr_to_string(addr: &sockaddr_un) -> String {
  // let sockname = &addr.sun_path[0..std::cmp::min(addr_len as usize, arr_size as usize)];
  let sockname = i8_slice_into_u8_slice(&addr.sun_path);
  let sockname = unsafe { str_from_u8_nul_utf8_unchecked(sockname) };

  sockname.to_string()
}

pub fn socket_addr_to_string(fd: i32) -> Result<String> {
  let mut addr = unsafe { mem::MaybeUninit::<sockaddr_un>::zeroed().assume_init() };
  let ty_size = mem::size_of::<sockaddr_un>() as u32;
  let mut addr_len = ty_size;
  resolve_libc_err(unsafe {
    libc::getsockname(
      fd,
      &mut addr as *mut _ as *mut sockaddr,
      &mut addr_len as *mut _,
    )
  })?;

  Ok(addr_to_string(&addr))
}

pub fn error(msg: String) -> Error {
  Error::new(napi::Status::Unknown, msg)
}

pub fn nix_err(err: Errno) -> Error {
  error(format!("operation failed, errno: {}", err))
}

pub fn resolve_uv_err(errno: i32) -> napi::Result<i32> {
  if errno >= 0 {
    return Ok(errno);
  }

  let msg = unsafe {
    let ret = sys::uv_err_name(errno);
    let ret = CStr::from_ptr(ret);
    ret
      .to_str()
      .map_err(|_| error("parsing cstr failed".to_string()))?
      .to_string()
  };

  Err(error(msg))
}

pub fn get_err() -> Error {
  let err = nix::errno::Errno::from_i32(nix::errno::errno());
  error(err.desc().to_string())
}

pub fn resolve_libc_err(ret: i32) -> napi::Result<i32> {
  if ret != -1 {
    return Ok(ret);
  }

  Err(get_err())
}

#[allow(dead_code)]
pub unsafe fn extend_life<'a, T>(e: &'a T) -> &'static T {
  transmute(e)
}

pub unsafe fn str_from_u8_nul_utf8_unchecked(utf8_src: &[u8]) -> &str {
  // does Rust have a built-in 'memchr' equivalent?
  let mut nul_range_end = 0_usize;
  for b in utf8_src {
    if *b == 0 {
      break;
    }
    nul_range_end += 1;
  }
  return ::std::str::from_utf8_unchecked(&utf8_src[0..nul_range_end]);
}

pub fn set_non_block(fd: i32) -> Result<()> {
  fcntl(fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).map_err(nix_err)?;
  Ok(())
}

pub fn set_clo_exec(fd: i32) -> Result<()> {
  fcntl(fd, FcntlArg::F_SETFL(OFlag::O_CLOEXEC)).map_err(nix_err)?;
  Ok(())
}
