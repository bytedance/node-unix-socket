use napi::{self, JsObject, Env, Error};
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

pub unsafe fn extend_life<'a, T>(e: &'a T) -> &'static T {
  transmute(e)
}
