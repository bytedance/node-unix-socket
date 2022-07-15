use napi::{self, Error};
use nix::errno::Errno;
use uv_sys::sys;
use std::ffi::CStr;
use std::intrinsics::transmute;

pub fn error(msg: String) -> Error {
  Error::new(napi::Status::Unknown, msg)
}

pub fn nix_err(err: Errno) -> Error {
  error(format!("operation failed, errno: {}", err))
}

pub fn resolve_uv_err(errno: i32) -> napi::Result<i32> {
  if errno >= 0 {
    return Ok(errno)
  }

  let msg = unsafe {
    let ret = sys::uv_err_name(errno);
    let ret = CStr::from_ptr(ret);
    ret.to_str().map_err(|_| {
      error("parsing cstr failed".to_string())
    })?.to_string()
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
  let mut nul_range_end = 1_usize;
  for b in utf8_src {
      if *b == 0 {
          break;
      }
      nul_range_end += 1;
  }
  return ::std::str::from_utf8_unchecked(&utf8_src[0..nul_range_end]);
}
