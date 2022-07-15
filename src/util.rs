use std::ffi::CStr;
use std::intrinsics::transmute;
use std::mem;

use libc::{sockaddr, sockaddr_un, c_char};
use napi::{self, Error, JsBuffer, Result, JsFunction, JsObject};
use nix::errno::Errno;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use uv_sys::sys;

pub(crate) fn i8_slice_into_u8_slice<'a>(slice: &'a [i8]) -> &'a [u8] {
  unsafe { &*(slice as *const [i8] as *const [u8]) }
}

pub(crate) fn addr_to_string(addr: &sockaddr_un) -> String {
  // sockaddr_un.sun_path/c_char has varied types in different types so that we transmute() it
  let path_ref: &[i8] = unsafe { transmute(&addr.sun_path as &[c_char]) };
  let sockname = i8_slice_into_u8_slice(path_ref);
  let sockname = unsafe { str_from_u8_nul_utf8_unchecked(sockname) };

  sockname.to_string()
}

pub(crate) fn socket_addr_to_string(fd: i32) -> Result<String> {
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

pub(crate) fn error(msg: String) -> Error {
  Error::new(napi::Status::Unknown, msg)
}

pub(crate) fn nix_err(err: Errno) -> Error {
  error(format!("operation failed, errno: {}", err))
}

pub(crate) fn uv_err_msg(errno: i32) -> String {
  let msg = unsafe {
    let ret = sys::uv_err_name(errno);
    let ret = CStr::from_ptr(ret);
    ret
      .to_str()
      .map_err(|_| error("parsing cstr failed".to_string())).unwrap()
      .to_string()
  };

  msg
}

pub(crate) fn uv_err(errno: i32) -> napi::Error {
  let msg = uv_err_msg(errno);
  error(msg)
}

pub(crate) fn resolve_uv_err(errno: i32) -> napi::Result<i32> {
  if errno >= 0 {
    return Ok(errno);
  }

  Err(uv_err(errno))
}

pub(crate) fn get_err() -> Error {
  let err = nix::errno::Errno::from_i32(nix::errno::errno());
  error(err.desc().to_string())
}

pub(crate) fn resolve_libc_err(ret: i32) -> napi::Result<i32> {
  if ret != -1 {
    return Ok(ret);
  }

  Err(get_err())
}

#[allow(dead_code)]
pub(crate) unsafe fn extend_life<'a, T>(e: &'a T) -> &'static T {
  transmute(e)
}

pub(crate) unsafe fn str_from_u8_nul_utf8_unchecked(utf8_src: &[u8]) -> &str {
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

pub(crate) fn set_non_block(fd: i32) -> Result<()> {
  fcntl(fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).map_err(nix_err)?;
  Ok(())
}

pub(crate) fn set_clo_exec(fd: i32) -> Result<()> {
  fcntl(fd, FcntlArg::F_SETFL(OFlag::O_CLOEXEC)).map_err(nix_err)?;
  Ok(())
}

pub(crate) fn buf_into_vec(buf: JsBuffer, offset: i32, length: i32) -> Result<Vec<u8>> {
  let buf = buf.into_value()?;
  let end = offset + length;
  let offset = offset as usize;
  let end = end as usize;

  Ok(buf[offset..end].to_vec())
}

pub(crate) fn check_emit(ee: &JsObject) -> Result<()> {
  let emit_fn = ee.get_named_property::<JsFunction>("emit");
  if emit_fn.is_err() {
    return Err(error("expect a js object with 'emit' function".to_string()));
  }

  Ok(())
}
