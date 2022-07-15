use std::mem;

use crate::socket::{close, get_loop, sockaddr_from_string};
use crate::util::{
  get_err, resolve_libc_err, resolve_uv_err, set_clo_exec, set_non_block, socket_addr_to_string,
  addr_to_string, error,
};
use libc::{sockaddr, sockaddr_un, EINVAL};
use napi::{Env, JsBuffer, JsFunction, JsNumber, JsObject, JsString, JsUnknown, Ref, Result};
use nix::errno::errno;
use uv_sys::sys;

#[napi]
pub struct SeqpacketSocketWrap {
  fd: i32,
  // TODO worker_threads
  env: Env,
  handle: *mut sys::uv_poll_t,
  connect_cb: Option<Ref<JsFunction>>,
  socket_cb: Option<Ref<JsFunction>>,
}

#[napi]
impl SeqpacketSocketWrap {
  #[napi(constructor)]
  pub fn new(env: Env, fd: Option<JsNumber>) -> Result<Self> {
    let domain = libc::AF_UNIX;
    // TODO
    // let ty = libc::SOCK_SEQPACKET;
    let ty = libc::SOCK_STREAM;
    let protocol = 0;
    let fd: i32 = match fd {
      Some(fd) => {
        fd.get_int32()?
      }
      None => {
        let fd = unsafe { libc::socket(domain, ty, protocol) };
        if fd == -1 {
          return Err(get_err());
        }
        fd
      }
    };

    set_non_block(fd)?;
    set_clo_exec(fd)?;

    // TODO reclaim
    let handle = Box::into_raw(Box::new(unsafe {
      mem::MaybeUninit::<sys::uv_poll_t>::zeroed().assume_init()
    }));

    let uv_loop = get_loop(&env)?;

    resolve_uv_err(unsafe { sys::uv_poll_init(uv_loop, handle, fd) })?;

    let wrap = Box::into_raw(Box::new(Self {
      fd,
      env,
      handle,
      connect_cb: None,
      socket_cb: None,
    }));

    unsafe { (*handle).data = wrap as *mut _ };

    Ok(unsafe { *Box::from_raw(wrap) })
  }

  #[napi]
  pub fn set_socket_cb(&mut self, _env: Env, cb: JsFunction) -> Result<()> {
    self.socket_cb = Some(cb.into_ref()?);
    Ok(())
  }

  #[napi]
  pub fn address(&self, env: Env) -> Result<JsString> {
    let str = socket_addr_to_string(self.fd)?;
    env.create_string(&str)
  }

  fn bind(&self, bindpath: &str) -> Result<()> {
    unsafe {
      let (sockaddr, _) = sockaddr_from_string(bindpath)?;
      resolve_libc_err(libc::bind(
        self.fd,
        &sockaddr as *const _ as *const sockaddr,
        mem::size_of::<sockaddr_un>() as u32,
      ))?;
    };

    Ok(())
  }

  fn handle_connect(&mut self, status: i32, events: i32) {
    // TODO
    assert!(status == 0, "receive unexpected status: {}", status);
    assert!(
      events & sys::uv_poll_event::UV_WRITABLE as i32 != 0,
      "receive unexpected events: {}",
      events
    );
    let env = self.env;

    // TODO
    let _ = env.run_in_scope(|| {
      self.connect_cb.as_mut().map(|cb| {
        cb.value(&env, |cb| {
          cb.call_without_args(None)?;
          Ok(())
        })
      });

      Ok(())
    });

    if self.connect_cb.is_some() {
      let mut cb = self.connect_cb.take().unwrap();
      // TODO
      cb.unref(env).unwrap();
    }
  }

  // TODO distinguish new conenction and read data
  fn handle_socket(&mut self, status: i32, events: i32) {
    // TODO
    assert!(status == 0, "receive unexpected status: {}", status);
    assert!(
      events & sys::uv_poll_event::UV_READABLE as i32 != 0,
      "receive unexpected events: {}",
      events
    );
    let mut addr = unsafe { mem::MaybeUninit::<sockaddr_un>::zeroed().assume_init() };
    let mut addr_len = mem::size_of::<sockaddr_un>() as u32;
    // TODO handle error
    let fd = resolve_libc_err(unsafe {
      libc::accept(
        self.fd,
        &mut addr as *mut _ as *mut libc::sockaddr,
        &mut addr_len as *mut _,
      )
    }).unwrap();

    assert!(!self.socket_cb.is_none(), "unexpected empty socket_cb");
    let env = self.env;
    let addr = addr_to_string(&addr);

    // TODO handle error
    let _ = env.run_in_scope(|| {
      // TODO emit socket
      let mut args: Vec<JsUnknown> = vec![];
      let js_fd = env.create_int32(fd)?;
      args.push(js_fd.into_unknown());
      let js_addr = env.create_string(&addr)?;
      args.push(js_addr.into_unknown());

      self.socket_cb.as_mut().map(|cb| {
        // TODO emit error
        let _ = cb.value(&env, |cb| {
          cb.call(None, &args)
        });
      });
      Ok(())
    });
  }

  #[napi]
  pub fn listen(&self, _env: Env, bindpath: JsString, backlog: JsNumber) -> Result<()> {
    if self.socket_cb.is_none() {
      return Err(error("expect setting socket_cb before listen()".to_string()))
    }
    // Should never call listen() with a fd for multiple times.
    let bindpath = bindpath.into_utf8()?;
    let backlog = backlog.get_int32()?;

    self.bind(bindpath.as_str()?)?;
    resolve_libc_err(unsafe { libc::listen(self.fd, backlog) })?;

    // start poll
    resolve_uv_err(unsafe {
      sys::uv_poll_start(
        self.handle,
        sys::uv_poll_event::UV_READABLE as i32,
        Some(on_socket),
      )
    })?;

    Ok(())
  }

  #[napi]
  pub fn connect(&mut self, _env: Env, server_path: JsString, cb: JsFunction) -> Result<()> {
    let server_path = server_path.into_utf8()?;
    let path = server_path.as_str()?;
    let (mut sockaddr, addr_len) = unsafe { sockaddr_from_string(path)? };
    let cb = cb.into_ref()?;
    self.connect_cb = Some(cb);

    let mut ret: i32;

    loop {
      ret = unsafe {
        libc::connect(
          self.fd,
          &mut sockaddr as *mut _ as *mut libc::sockaddr,
          addr_len as u32,
        )
      };

      if !(ret == -1 && ret == libc::EINTR) {
        break;
      }
    }

    let err = errno();

    if ret == -1 && err != 0 {
      if err == libc::EINPROGRESS {
        // not an error
      } else if err == libc::ECONNRESET || err == EINVAL {
        // TODO should we delay error?
        resolve_libc_err(ret)?;
      } else {
        resolve_libc_err(ret)?;
      }
    }

    // TODO test close before connected
    unsafe {
      sys::uv_poll_start(
        self.handle,
        sys::uv_poll_event::UV_WRITABLE as i32,
        Some(on_connect),
      )
    };

    Ok(())
  }

  #[napi]
  pub fn write(&self, env: Env, buf: JsBuffer, cb: JsFunction) -> Result<()> {
    // TODO
    Ok(())
  }

  #[napi]
  pub fn close(&mut self, env: Env) -> Result<()> {
    // close handle
    resolve_uv_err(unsafe { sys::uv_poll_stop(self.handle) })?;
    unsafe {
      (*(self.handle)).data = std::ptr::null_mut();
      let handle = mem::transmute(self.handle);
      sys::uv_close(handle, Some(on_close));
    };

    // TODO unref other cb
    let mut socket_cb = self.socket_cb.take();
    match socket_cb.as_mut() {
      None => {}
      Some(cb) => {
        cb.unref(env)?;
      }
    }

    close(self.fd)
  }
}

extern "C" fn on_socket(
  handle: *mut sys::uv_poll_t,
  status: ::std::os::raw::c_int,
  events: ::std::os::raw::c_int,
) {
  let mut wrap = unsafe { Box::from_raw((*handle).data as *mut SeqpacketSocketWrap) };
  wrap.handle_socket(status, events);
  Box::leak(wrap);
}

extern "C" fn on_close(handle: *mut sys::uv_handle_t) {
  unsafe {
    Box::from_raw(handle);
  };
}

extern "C" fn on_connect(
  handle: *mut sys::uv_poll_t,
  status: ::std::os::raw::c_int,
  events: ::std::os::raw::c_int,
) {
  let mut wrap = unsafe { Box::from_raw((*handle).data as *mut SeqpacketSocketWrap) };
  wrap.handle_connect(status, events);
  Box::leak(wrap);
}
