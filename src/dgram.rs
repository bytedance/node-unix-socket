use std::mem;
use std::{
  collections::LinkedList, ffi::CString, intrinsics::transmute, str::FromStr,
};

use libc::{
  self, c_void, iovec, msghdr, sockaddr, sockaddr_un,
  EAGAIN, EINPROGRESS, EINTR, ENOBUFS, EWOULDBLOCK,
};
use napi::{Env, JsBuffer, JsFunction, JsObject, JsUnknown, Ref, Result};
use nix::{
  self,
  errno::errno,
  fcntl::{fcntl, FcntlArg, OFlag},
};
use uv_sys::sys::{self, uv_poll_event};

use crate::util::{
  error, get_err, nix_err, resolve_libc_err, resolve_uv_err,
  str_from_u8_nul_utf8_unchecked,
};

fn set_non_block(fd: i32) -> Result<()> {
  fcntl(fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).map_err(nix_err)?;
  Ok(())
}

fn set_clo_exec(fd: i32) -> Result<()> {
  fcntl(fd, FcntlArg::F_SETFL(OFlag::O_CLOEXEC)).map_err(nix_err)?;
  Ok(())
}

fn get_loop(env: &Env) -> Result<*mut sys::uv_loop_t> {
  Ok(env.get_uv_event_loop()? as *mut _ as *mut sys::uv_loop_t)
}

fn i8_slice_into_u8_slice<'a>(slice: &'a [i8]) -> &'a [u8] {
  unsafe { &*(slice as *const [i8] as *const [u8]) }
}

#[allow(dead_code)]
fn string_from_i8_slice(slice: &[i8]) -> Result<String> {
  let trans = i8_slice_into_u8_slice(slice);
  let mut copy: Vec<u8> = vec![0; trans.len()];
  copy.clone_from_slice(trans);

  String::from_utf8(copy).map_err(|_| error("failed to parse i8 slice as string".to_string()))
}

pub fn on_readable(s: &Box<DgramSocketWrap>) -> Result<()> {
  let mut msg = unsafe { mem::MaybeUninit::<msghdr>::zeroed().assume_init() };
  let cap = 65535;
  let base = unsafe { CString::from_vec_unchecked(vec![0; cap]) };
  let base_ptr = base.into_raw();

  let mut iov = libc::iovec {
    iov_base: base_ptr as *mut _,
    iov_len: cap,
  };

  let mut name = unsafe { mem::MaybeUninit::<sockaddr_un>::zeroed().assume_init() };
  let name_len = mem::size_of::<sockaddr_un>();
  msg.msg_iovlen = 1;
  msg.msg_iov = &mut iov as *mut _;
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
    unsafe { Box::from_raw(base_ptr) };

    let err = error(format!("recv msg failed, errno: {}", errno()));
    let err = env.create_error(err)?;
    args.push(err.into_unknown());

    let _ = s
      .recv_cb
      .value(&s.env, |cb| cb.call(None, &args))
      .map_err(|e| {
        let _ = env.throw_error(&e.reason, None);
      });
  } else {
    let iov = unsafe { *msg.msg_iov };
    // NOTE: Vec::from_raw_parts will consum the ptr and respond to reclaim it.
    // TODO not safe
    let iov_base = unsafe { Vec::from_raw_parts(iov.iov_base as *mut u8, iov.iov_len, iov.iov_len) };
    let len = ret as usize;
    let slice = iov_base[0..len].to_vec();

    let name = unsafe { *(msg.msg_name as *mut sockaddr_un) };

    let js_sockname = {
      let len = name.sun_len as usize;
      let sockname = &name.sun_path[0..std::cmp::min(len, name.sun_path.len())];
      let sockname = i8_slice_into_u8_slice(sockname);
      let sockname = unsafe { str_from_u8_nul_utf8_unchecked(sockname) };
      env.create_string(&sockname[0..sockname.len() - 1])?
    };

    let buf = env.create_buffer_with_data(slice)?;
    args.push(env.get_undefined()?.into_unknown());
    args.push(buf.into_unknown());
    args.push(js_sockname.into_unknown());

    let _ = s
      .recv_cb
      .value(&s.env, |cb| cb.call(None, &args))
      .map_err(|e| {
        let _ = env.throw_error(&e.reason, None);
      });
  }

  Ok(())
}
pub fn on_writable(s: &mut Box<DgramSocketWrap>) -> Result<()> {
  s.flush()
}

unsafe fn get_socket(data: *mut c_void) -> Box<DgramSocketWrap> {
  let ctx: *mut DgramSocketWrap = transmute(data);
  Box::from_raw(ctx)
}

extern "C" fn on_event(handle: *mut sys::uv_poll_t, status: i32, events: i32) {
  let handle = unsafe { Box::from_raw(handle) };
  assert!(!handle.data.is_null(), "'on_event' receive null_ptr handle data");
  let mut socket = unsafe { get_socket(handle.data) };
  let env = socket.env;

  /*
   * FIXME(oyyd): env.run_in_scope() below might produce an EXC_BAD_ACCESS error.
   */
  let socket = env.run_in_scope(move || {
    if status != 0 {
      let _ = env.throw_error(&format!("on_event receive error status: {}", status), None);
      return Ok(socket)
    }

    if events & uv_poll_event::UV_READABLE as i32 != 0 {
      on_readable(&socket).map_err(|e| {
        let _ = env.throw_error(&e.reason, None);
        e
      }).or::<napi::Error>(Ok(())).unwrap();
    }

    if events & uv_poll_event::UV_WRITABLE as i32 != 0 {
      on_writable(&mut socket).map_err(|e| {
        let _ = env.throw_error(&e.reason, None);
        e
      }).or::<napi::Error>(Ok(())).unwrap();
    }

    Ok(socket)
  }).unwrap();

  Box::into_raw(socket);
  Box::into_raw(handle);
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

#[napi]
pub struct DgramSocketWrap {
  fd: i32,
  // TODO env might be invalid
  env: Env,
  // handle should be freed on on_close
  handle: *mut sys::uv_poll_t,
  recv_cb: Ref<JsFunction>,
  msg_queue: LinkedList<MsgItem>,
  this: Option<Ref<JsObject>>,
}

#[napi]
impl DgramSocketWrap {
  #[napi(constructor)]
  pub fn new(env: Env, recv_cb: JsFunction) -> Result<Self> {
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

    let handle = Box::into_raw(Box::new(unsafe {
      mem::MaybeUninit::<sys::uv_poll_t>::zeroed().assume_init()
    }));
    let socket = Box::into_raw(Box::new(DgramSocketWrap {
      fd,
      handle,
      msg_queue: LinkedList::new(),
      env,
      recv_cb: recv_cb.into_ref()?,
      this: None,
    }));

    unsafe {
      (*handle).data = socket as *mut _;
    }

    Ok(unsafe { *Box::from_raw(socket) })
  }

  #[napi]
  pub fn start_recv(&mut self, env: Env) -> Result<()> {
    let uv_loop = get_loop(&env)?;

    unsafe {
      resolve_uv_err(sys::uv_poll_init(uv_loop, self.handle, self.fd))?;
      resolve_uv_err(sys::uv_poll_start(
        self.handle,
        sys::uv_poll_event::UV_READABLE as i32,
        Some(on_event),
      ))?;
    }

    Ok(())
  }

  /**
   * NOTE: Because we can't get the "this" js object of DgramSocketWrap instances,
   * we need to call ref_this manually in the js side to prevent the js object
   * from been garbage-collected.
   *
   * TODO Is there a way to get the js object in rust side?
   */
  #[napi]
  pub fn ref_this(&mut self, this_obj: JsObject) -> Result<()> {
    let this = this_obj.into_ref()?;
    self.this = Some(this);

    Ok(())
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

      unsafe { Box::from_raw(iov.iov_base) };

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
      let _ = item
        .cb
        .value(&self.env, |cb| cb.call(None, &args))
        .map_err(|e| {
          let _ = self.env.throw_error(&e.reason, None);
        });
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

    self.flush()?;

    Ok(())
  }

  #[napi]
  pub fn close(&mut self, env: Env) -> Result<()> {
    // stop watcher
    resolve_uv_err(unsafe { sys::uv_poll_stop(self.handle) })?;
    unsafe {
      (*(self.handle)).data = std::ptr::null_mut();
      let handle = mem::transmute(self.handle);
      sys::uv_close(handle, Some(on_close));
    };

    self.recv_cb.unref(env)?;
    match self.this.as_mut() {
      None => {}
      Some(this) => {
        this.unref(env)?;
      }
    }

    let ret = unsafe { libc::close(self.fd) };

    if ret != 0 {
      if ret != EINTR && ret != EINPROGRESS {
        return Err(get_err());
      }
    }

    return Ok(());
  }
}
