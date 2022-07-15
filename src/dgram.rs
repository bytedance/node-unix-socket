use std::collections::LinkedList;
use std::mem;

use libc::{
  self, c_void, iovec, msghdr, sockaddr, sockaddr_un, EAGAIN, EINTR, ENOBUFS, EWOULDBLOCK,
};
use napi::{Env, JsBuffer, JsFunction, JsNumber, JsObject, JsString, JsUnknown, Ref, Result};
use nix::{self, errno::errno};
use uv_sys::sys::{self, uv_poll_event};

use crate::socket::{close, get_loop, sockaddr_from_string, Emitter, HandleData};
use crate::util::{
  addr_to_string, buf_into_vec, check_emit, error, get_err, i8_slice_into_u8_slice,
  resolve_libc_err, resolve_uv_err, set_clo_exec, set_non_block, socket_addr_to_string,
};

fn unwrap<'a>(env: &'a Env, this: &JsObject) -> Result<&'a mut DgramSocketWrap> {
  env.unwrap(&this)
}

#[allow(dead_code)]
#[napi]
pub fn dgram_create_socket(env: Env, ee: JsObject) -> Result<()> {
  check_emit(&ee)?;
  DgramSocketWrap::wrap(env, ee)?;
  Ok(())
}

#[allow(dead_code)]
#[napi]
pub fn dgram_start_recv(env: Env, ee: JsObject) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  wrap.start_recv(env)?;
  Ok(())
}

#[allow(dead_code)]
#[napi]
pub fn dgram_close(env: Env, ee: JsObject) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  wrap.close(env)?;
  Ok(())
}

#[allow(dead_code)]
#[napi]
pub fn dgram_bind(env: Env, ee: JsObject, bindpath: String) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  wrap.bind(bindpath)?;
  Ok(())
}

#[allow(dead_code)]
#[napi]
pub fn dgram_address(env: Env, ee: JsObject) -> Result<JsString> {
  let wrap = unwrap(&env, &ee)?;
  wrap.address(env)
}

#[allow(dead_code)]
#[napi]
pub fn dgram_get_recv_buffer_size(env: Env, ee: JsObject) -> Result<JsNumber> {
  let wrap = unwrap(&env, &ee)?;
  wrap.get_recv_buffer_size(env)
}

#[allow(dead_code)]
#[napi]
pub fn dgram_set_recv_buffer_size(env: Env, ee: JsObject, size: JsNumber) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  wrap.set_recv_buffer_size(size)
}

#[allow(dead_code)]
#[napi]
pub fn dgram_get_send_buffer_size(env: Env, ee: JsObject) -> Result<JsNumber> {
  let wrap = unwrap(&env, &ee)?;
  wrap.get_send_buffer_size(env)
}

#[allow(dead_code)]
#[napi]
pub fn dgram_set_send_buffer_size(env: Env, ee: JsObject, size: JsNumber) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  wrap.set_send_buffer_size(size)
}

#[allow(dead_code)]
#[napi]
pub fn dgram_send_to(
  env: Env,
  ee: JsObject,
  buf: JsBuffer,
  offset: JsNumber,
  length: JsNumber,
  path: String,
  cb: Option<JsFunction>,
) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  wrap.send_to(env, buf, offset, length, path, cb)
}

#[allow(dead_code)]
fn string_from_i8_slice(slice: &[i8]) -> Result<String> {
  let trans = i8_slice_into_u8_slice(slice);
  let mut copy: Vec<u8> = vec![0; trans.len()];
  copy.clone_from_slice(trans);

  String::from_utf8(copy).map_err(|_| error("failed to parse i8 slice as string".to_string()))
}

struct MsgItem {
  msg: Vec<u8>,
  sockaddr: sockaddr_un,
  cb: Option<Ref<()>>,
}

pub struct DgramSocketWrap {
  fd: i32,
  env: Env,
  handle: *mut sys::uv_poll_t,
  msg_queue: LinkedList<MsgItem>,
  emitter: Emitter,
}

impl DgramSocketWrap {
  fn wrap(env: Env, mut this: JsObject) -> Result<()> {
    let domain = libc::AF_UNIX;
    let ty = libc::SOCK_DGRAM;
    let protocol = 0;
    let fd = unsafe { libc::socket(domain, ty, protocol) };

    if fd == -1 {
      return Err(get_err());
    }

    set_non_block(fd)?;
    set_clo_exec(fd)?;

    let emit_fn: JsFunction = this.get_named_property("emit")?;
    let handle = Box::into_raw(Box::new(unsafe {
      mem::MaybeUninit::<sys::uv_poll_t>::zeroed().assume_init()
    }));
    let socket = DgramSocketWrap {
      fd,
      handle,
      msg_queue: LinkedList::new(),
      env,
      emitter: Emitter::new(env, emit_fn)?,
    };

    env.wrap(&mut this, socket)?;
    let data = Box::into_raw(Box::new(HandleData::new(env, this)?));
    unsafe {
      (*handle).data = data as *mut _;
    }

    Ok(())
  }

  fn start_recv(&mut self, env: Env) -> Result<()> {
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

  pub fn address(&self, env: Env) -> Result<JsString> {
    let str = socket_addr_to_string(self.fd)?;
    env.create_string(&str)
  }

  pub fn get_recv_buffer_size(&self, env: Env) -> Result<JsNumber> {
    let mut val = 0_i32;
    let mut len = mem::size_of::<i32>() as u32;
    resolve_libc_err(unsafe {
      libc::getsockopt(
        self.fd,
        libc::SOL_SOCKET,
        libc::SO_RCVBUF,
        &mut val as *mut _ as *mut c_void,
        &mut len as *mut _,
      )
    })?;
    env.create_int32(val)
  }

  pub fn set_recv_buffer_size(&self, size: JsNumber) -> Result<()> {
    let mut val = size.get_uint32()?;
    let len = mem::size_of::<i32>() as u32;
    resolve_libc_err(unsafe {
      libc::setsockopt(
        self.fd,
        libc::SOL_SOCKET,
        libc::SO_RCVBUF,
        &mut val as *mut _ as *mut c_void,
        len,
      )
    })?;
    Ok(())
  }

  pub fn get_send_buffer_size(&self, env: Env) -> Result<JsNumber> {
    let mut val = 0_i32;
    let mut len = mem::size_of::<i32>() as u32;
    resolve_libc_err(unsafe {
      libc::getsockopt(
        self.fd,
        libc::SOL_SOCKET,
        libc::SO_SNDBUF,
        &mut val as *mut _ as *mut c_void,
        &mut len as *mut _,
      )
    })?;
    env.create_int32(val)
  }

  pub fn set_send_buffer_size(&self, size: JsNumber) -> Result<()> {
    let mut val = size.get_uint32()?;
    let len = mem::size_of::<i32>() as u32;
    resolve_libc_err(unsafe {
      libc::setsockopt(
        self.fd,
        libc::SOL_SOCKET,
        libc::SO_SNDBUF,
        &mut val as *mut _ as *mut c_void,
        len,
      )
    })?;
    Ok(())
  }

  fn flush(&mut self) -> Result<()> {
    let env = self.env;
    loop {
      let item = self.msg_queue.pop_front();
      if item.is_none() {
        break;
      }
      let mut item = item.unwrap();
      let mut msg = unsafe { mem::MaybeUninit::<msghdr>::zeroed().assume_init() };
      let mut iov = unsafe { mem::MaybeUninit::<iovec>::zeroed().assume_init() };
      let len = item.msg.len();

      iov.iov_base = item.msg.as_mut_ptr() as *mut _;
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
        // TODO is this a unrecoverable error?
        let err = self.env.create_error(get_err())?;
        args.push(err.into_unknown());
      }

      // call callbacks
      if item.cb.is_some() {
        let cb_ref = item.cb.as_mut().take().unwrap();
        let cb: JsFunction = env.get_reference_value(&cb_ref)?;
        let _ = cb.call(None, &args).map_err(|e| {
          let _ = self.env.throw_error(&e.reason, None);
        });
        cb_ref.unref(self.env)?;
      }
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

  pub fn send_to(
    &mut self,
    env: Env,
    buf: JsBuffer,
    offset: JsNumber,
    length: JsNumber,
    path: String,
    cb: Option<JsFunction>,
  ) -> Result<()> {
    let offset = offset.get_int32()?;
    let length = length.get_int32()?;
    let end = offset + length;
    let offset = offset;
    let end = end;
    let msg = buf_into_vec(buf, offset, end)?;

    let (addr, _) = sockaddr_from_string(&path)?;
    let cb = match cb {
      None => None,
      Some(cb) => Some(env.create_reference(cb)?),
    };

    let m = MsgItem {
      sockaddr: addr,
      msg,
      cb,
    };

    self.msg_queue.push_back(m);

    self.flush()?;

    Ok(())
  }

  pub fn close(&mut self, env: Env) -> Result<()> {
    // stop watcher
    let is_closing = unsafe { sys::uv_is_closing(self.handle as *mut _) } != 0;
    if !is_closing {
      resolve_uv_err(unsafe { sys::uv_poll_stop(self.handle) })?;
    }
    unsafe {
      let handle = mem::transmute(self.handle);
      sys::uv_close(handle, Some(on_close));
    };

    // release Ref<JsFunction> in msg_queue
    loop {
      let msg = self.msg_queue.pop_front();
      if msg.is_none() {
        break;
      }

      let mut msg = msg.unwrap();
      let mut cb = msg.cb.take();
      match cb.as_mut() {
        None => (),
        Some(cb) => {
          cb.unref(env)?;
        }
      }
    }

    close(self.fd)?;

    let event = env.create_string("close")?;
    self.emitter.emit(&[event.into_unknown()])?;
    self.emitter.unref()?;

    Ok(())
  }

  fn read_data(&mut self) -> Result<()> {
    let s = self;
    loop {
      let mut msg = unsafe { mem::MaybeUninit::<msghdr>::zeroed().assume_init() };
      let cap = 65535;
      let mut base = vec![0; cap];
      let base_ptr = base.as_mut_ptr();

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
      let env = s.env.clone();

      if ret == -1 {
        let err = errno();
        if err == EAGAIN || err == EWOULDBLOCK || err == ENOBUFS {
          break;
        }
        let event = env.create_string("_error")?;
        args.push(event.into_unknown());
        let err = error(format!("recv msg failed, errno: {}", errno()));
        let err = env.create_error(err)?;
        args.push(err.into_unknown());
      } else {
        let len = ret as usize;
        let slice = base[0..len].to_vec();

        let name = unsafe { *(msg.msg_name as *mut sockaddr_un) };

        let js_sockname = {
          let name = addr_to_string(&name);
          env.create_string(&name)?
        };

        let buf = env.create_buffer_with_data(slice)?;
        let event = env.create_string("_data")?;
        args.push(event.into_unknown());
        args.push(buf.into_unknown());
        args.push(js_sockname.into_unknown());
      }

      let _ = s.emitter.emit(&args).map_err(|e| {
        let _ = env.throw_error(&e.reason, None);
      });
    }

    Ok(())
  }

  fn handle_event(&mut self, status: i32, events: i32) {
    if status == sys::uv_errno_t::UV_ECANCELED as i32 {
      return;
    }

    let env = self.env;
    env
      .run_in_scope(|| {
        if status != 0 {
          let _ = env.throw_error(&format!("on_event receive error status: {}", status), None);
          return Ok(());
        }

        if events & uv_poll_event::UV_READABLE as i32 != 0 {
          self
            .read_data()
            .map_err(|e| {
              let _ = env.throw_error(&e.reason, None);
              e
            })
            .or::<napi::Error>(Ok(()))
            .unwrap();
        }

        if events & uv_poll_event::UV_WRITABLE as i32 != 0 {
          self
            .flush()
            .map_err(|e| {
              let _ = env.throw_error(&e.reason, None);
              e
            })
            .or::<napi::Error>(Ok(()))
            .unwrap();
        }

        Ok(())
      })
      .unwrap();
  }
}

extern "C" fn on_event(handle: *mut sys::uv_poll_t, status: i32, events: i32) {
  let handle = unsafe { Box::from_raw(handle) };
  let data = unsafe { Box::from_raw(handle.data as *mut HandleData) };

  let wrap: &mut DgramSocketWrap = data.inner_mut_ref().unwrap();
  wrap.handle_event(status, events);

  Box::into_raw(data);
  Box::into_raw(handle);
}

extern "C" fn on_close(handle: *mut sys::uv_handle_t) {
  unsafe {
    let handle = Box::from_raw(handle);
    let mut data = Box::from_raw(handle.data as *mut HandleData);
    data.unref().unwrap();
  };
}
