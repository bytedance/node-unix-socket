use std::{
  borrow::BorrowMut, collections::LinkedList, ffi::CString, intrinsics::transmute, io::Write,
  str::FromStr,
};

use crate::util::{error, extend_life, get_err, nix_err, resolve_libc_err, resolve_uv_err};
use libc::{
  self, c_void, iovec, msghdr, sockaddr, sockaddr_un, EAGAIN, EINPROGRESS, EINTR, ENOBUFS,
  EWOULDBLOCK,
};
use napi::{Env, JsBuffer, JsFunction, JsUnknown, Ref, Result};
use nix::{
  self,
  errno::errno,
  fcntl::{fcntl, FcntlArg, OFlag},
};
use std::mem;
use uv_sys::sys::{self, uv_poll_event};

fn set_non_block(fd: i32) -> Result<()> {
  fcntl(fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).map_err(nix_err)?;
  Ok(())
}

fn set_clo_exec(fd: i32) -> Result<()> {
  fcntl(fd, FcntlArg::F_SETFL(OFlag::O_CLOEXEC)).map_err(nix_err)?;
  Ok(())
}

fn get_loop(env: &Env) -> Result<*mut sys::uv_loop_t> {
  unsafe {
    let uv_loop = env.get_uv_event_loop()?;
    let uv_loop: Box<uv_sys::sys::uv_loop_t> = Box::from_raw(uv_loop as *mut _);
    Ok(Box::into_raw(uv_loop))
  }
}

fn string_from_i8_slice(slice: &[i8]) -> Result<String> {
  let trans = unsafe { &*(slice as *const [i8] as *const [u8]) };
  let copy: Vec<u8> = vec![0; trans.len()];

  String::from_utf8(copy).map_err(|_| error("failed to parse i8 slice as string".to_string()))
}

pub fn on_readable(s: &Box<DgramSocketWrap>) -> Result<()> {
  let mut msg = unsafe { mem::MaybeUninit::<msghdr>::zeroed().assume_init() };
  let cap = 65535;
  let base = unsafe { CString::from_vec_unchecked(vec![0; cap]) };

  let iov = libc::iovec {
    iov_base: base.into_raw() as *mut _,
    iov_len: cap,
  };
  let iov = Box::into_raw(Box::new(iov));

  // TODO check sockadd length
  let mut name = unsafe { mem::MaybeUninit::<sockaddr_un>::zeroed().assume_init() };
  let name_len = mem::size_of::<sockaddr_un>();
  msg.msg_iovlen = 1;
  msg.msg_iov = iov;
  msg.msg_name = &mut name as *mut sockaddr_un as *mut _;
  msg.msg_namelen = name_len as u32;

  let mut ret;
  loop {
    ret = unsafe { libc::recvmsg(s.fd, &mut msg as *mut _, 0) };
    if !(ret == -1 && errno() == nix::Error::EINTR as i32) {
      break;
    }
  }

  let mut args: Vec<JsUnknown> = vec![];
  let env = &s.env;

  if ret == -1 {
    let err = error(format!("recv msg failed, errno: {}", errno()));
    let err = env.create_error(err)?;
    args.push(err.into_unknown());
    s.recv_cb.value(&s.env, |cb| cb.call(None, &args))?;
  } else {
    let iov = unsafe { *msg.msg_iov };
    let iov_base = unsafe { CString::from_raw(iov.iov_base as *mut i8) };
    let name = unsafe { *(msg.msg_name as *mut sockaddr_un) };

    let len = name.sun_len as usize;
    let sockname = &name.sun_path[0..len];

    let len = ret as usize;

    let slice = iov_base.as_bytes()[0..len].to_vec();
    let buf = env.create_buffer_with_data(slice)?;
    let sockname = string_from_i8_slice(sockname)?;
    let sockname = env.create_string_from_std(sockname)?;
    args.push(env.get_undefined()?.into_unknown());
    args.push(buf.into_unknown());
    args.push(sockname.into_unknown());

    println!("call_a");
    s.recv_cb.value(&s.env, |cb| cb.call(None, &args))?;
    println!("end_call_a");
  }

  Ok(())
}

pub fn on_writable(s: &mut Box<DgramSocketWrap>) -> Result<()> {
  let env = s.clone_env();

  env.run_in_scope(move || {
    s.flush()?;
    Ok(())
  })?;

  Ok(())
}

unsafe fn get_socket(data: *mut c_void) -> Box<DgramSocketWrap> {
  let ctx: *mut DgramSocketWrap = transmute(data);
  Box::from_raw(ctx)
}

extern "C" fn on_event(handle: *mut sys::uv_poll_t, status: i32, events: i32) {
  println!("begin run_in_scope");

  let mut socket = unsafe { get_socket((*handle).data) };
  let env = socket.env.clone();

  // TODO
  if status != 0 {
    panic!("on_event receive error status");
  }

  println!("before run_in_scope");
  let socket = env
    .run_in_scope(move || {
      println!("a run_in_scope");
      if events & uv_poll_event::UV_READABLE as i32 != 0 {
        on_readable(&socket)?;
        // TODO should poll readable again?
      }
      println!("b run_in_scope");

      if events & uv_poll_event::UV_WRITABLE as i32 != 0 {
        on_writable(&mut socket)?;
      }
      println!("c run_in_scope");

      println!("d run_in_scope");

      Ok(socket)
    })
    .unwrap();

  println!("finish run_in_scope");
  Box::into_raw(socket);
}

extern "C" fn on_close(handle: *mut sys::uv_handle_t) {
  unsafe {
    Box::from_raw(handle);
  };
}

unsafe fn sockaddr_from_string(bytes: &str) -> Result<sockaddr_un> {
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

  Ok(sockaddr)
}

struct MsgItem {
  msg: Vec<u8>,
  sockaddr: sockaddr_un,
  // NOTE: Always remeber to free a Ref
  cb: Ref<JsFunction>,
}

impl Drop for DgramSocketWrap {
  fn drop(&mut self) {
    println!("drop DgramSocketWrap");
  }
}

#[napi]
pub struct DgramSocketWrap {
  fd: i32,
  // TODO env might be invalid
  env: Env,
  // handle is a raw pointer and will be freed on on_close
  handle: *mut sys::uv_poll_t,
  recv_cb: Ref<JsFunction>,
  msg_queue: LinkedList<MsgItem>,
}

#[napi]
impl DgramSocketWrap {
  #[napi(constructor)]
  pub fn new(env: Env, recv_cb: napi::JsFunction) -> Result<Self> {
    let domain = libc::AF_UNIX;
    let ty = libc::SOCK_DGRAM;
    let protocol = 0;

    let fd = unsafe { libc::socket(domain, ty, protocol) };

    if fd == -1 {
      let errno = -nix::errno::errno();
      return Err(error(format!("failed to create socket, errno: {}", errno)));
    }

    set_non_block(fd)?;
    set_clo_exec(fd)?;

    // TODO use MaybeUninit instead
    let handle = Box::into_raw(Box::new(sys::uv_poll_t::default()));
    let socket = Box::into_raw(Box::new(DgramSocketWrap {
      fd,
      handle,
      msg_queue: LinkedList::new(),
      env,
      recv_cb: recv_cb.into_ref()?,
    }));

    unsafe {
      (*handle).data = socket as *mut _;
    }

    // start watcher
    let uv_loop = get_loop(&env)?;

    unsafe {
      resolve_uv_err(sys::uv_poll_init(uv_loop, handle, fd))?;
      resolve_uv_err(sys::uv_poll_start(
        handle,
        sys::uv_poll_event::UV_READABLE as i32,
        Some(on_event),
      ))?;
    }

    Ok(unsafe { *Box::from_raw(socket) })
  }

  #[napi]
  pub fn bind(&self, bindpath: String) -> Result<()> {
    unsafe {
      let sockaddr = sockaddr_from_string(&bindpath)?;
      resolve_libc_err(libc::bind(
        self.fd,
        &sockaddr as *const _ as *const sockaddr,
        mem::size_of::<sockaddr_un>() as u32,
      ))?;
    };

    Ok(())
  }

  fn flush(&mut self) -> Result<()> {
    loop {
      let item = self.msg_queue.pop_front();
      if item.is_none() {
        break;
      }
      let mut item = item.unwrap();
      let mut msg = unsafe { mem::MaybeUninit::<msghdr>::zeroed().assume_init() };
      let mut iov = unsafe { mem::MaybeUninit::<iovec>::zeroed().assume_init() };
      let len = item.msg.len();
      let base = unsafe { CString::from_vec_unchecked(item.msg.clone()) };

      iov.iov_base = base.into_raw() as *mut _;
      iov.iov_len = len;

      msg.msg_iovlen = 1;
      msg.msg_iov = &mut iov as *mut _;
      msg.msg_name = &mut item.sockaddr as *mut sockaddr_un as *mut _;
      msg.msg_namelen = mem::size_of::<sockaddr_un>() as u32;

      let mut ret;
      loop {
        ret = unsafe { libc::sendmsg(self.fd, &mut msg as *mut _, 0) as i32 };

        if !(ret == -1 && errno() == EINTR) {
          break;
        }
      }

      let mut args: Vec<JsUnknown> = vec![];
      if ret == -1 {
        let err = errno();
        if err == EAGAIN || err == EWOULDBLOCK || err == ENOBUFS {
          self.msg_queue.push_front(item);
          break;
        }
        // TODO emit error and stop sending more message
        let err = self.env.create_error(get_err())?;
        args.push(err.into_unknown());
      }

      // callback sendmsg successfully
      item.cb.value(&self.env, |cb| cb.call(None, &args))?;

      item.cb.unref(self.env)?;
    }

    // poll writable if there are messages
    if self.msg_queue.len() > 0 {
      unsafe {
        resolve_uv_err(sys::uv_poll_start(
          self.handle,
          sys::uv_poll_event::UV_WRITABLE as i32,
          Some(on_event),
        ))?;
      };
    }

    Ok(())
  }

  /**
   * buf, offset, length, path
   */
  #[napi]
  pub fn send_to(
    &mut self,
    buf: JsBuffer,
    offset: i32,
    length: i32,
    path: String,
    cb: JsFunction,
  ) -> Result<()> {
    let buf = buf.into_value()?;
    let end = offset + length;
    let offset = offset as usize;
    let end = end as usize;

    let addr = unsafe { sockaddr_from_string(&path)? };

    let m = MsgItem {
      sockaddr: addr,
      msg: buf[offset..end].to_vec(),
      cb: cb.into_ref()?,
    };

    self.msg_queue.push_back(m);

    // TODO add test
    self.flush()?;

    Ok(())
  }

  #[napi]
  pub fn close(&mut self) -> Result<()> {
    let ret = unsafe { libc::close(self.fd) };
    if ret != 0 {
      if ret != EINTR && ret != EINPROGRESS {
        // TODO update error message
        return Err(error(format!("close failed, code: {}", -ret)));
      }
    }
    // stop watcher
    resolve_uv_err(unsafe { sys::uv_poll_stop(self.handle) })?;
    unsafe {
      let handle = mem::transmute(self.handle);
      sys::uv_close(handle, Some(on_close));
    };
    self.recv_cb.unref(self.env)?;

    return Ok(());
  }

  fn clone_env(&self) -> Env {
    self.env.clone()
  }
}
