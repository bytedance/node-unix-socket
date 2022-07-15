use std::collections::LinkedList;
use std::mem;
use std::os::raw::c_int;

use crate::socket::{self, get_loop, sockaddr_from_string};
use crate::util::{
  addr_to_string, buf_into_vec, error, get_err, resolve_libc_err, resolve_uv_err, set_clo_exec,
  set_non_block, socket_addr_to_string, uv_err_msg, check_emit,
};
use libc::{sockaddr, sockaddr_un, EAGAIN, EINTR, EINVAL, ENOBUFS, EWOULDBLOCK};
use napi::{Env, JsBuffer, JsFunction, JsNumber, JsObject, JsString, JsUnknown, Ref, Result};
use nix::errno::errno;
use uv_sys::sys;

const DEFAULT_READ_BUF_SIZE: usize = 65535;

#[derive(Eq, Ord, PartialEq, PartialOrd, Copy, Clone)]
enum State {
  NewSocket = 1,
  ShuttingDown = 2,
  ShutDown = 3,
  // Stopped = 4,
  Closed = 5,
}

struct HandleData {
  env: Env,
  this_ref: Ref<()>,
}

struct MsgItem {
  msg: Vec<u8>,
  cb: Option<Ref<()>>,
}

struct SeqpacketSocketWrap {
  fd: i32,
  // TODO worker_threads
  env: Env,
  handle: *mut sys::uv_poll_t,
  msg_queue: LinkedList<MsgItem>,
  read_buf_size: usize,
  state: State,
  poll_events: i32,
  emit_ref: Ref<()>,
}

fn unwrap<'a>(env: &'a Env, this: &JsObject) -> Result<&'a mut SeqpacketSocketWrap> {
  let wrap: &mut SeqpacketSocketWrap = env.unwrap(&this)?;
  Ok(wrap)
}

#[allow(dead_code)]
#[napi]
pub fn seq_create_socket(env: Env, ee: JsObject, fd: Option<JsNumber>) -> Result<()> {
  check_emit(&ee)?;
  SeqpacketSocketWrap::wrap_obj(env, ee, fd)?;
  Ok(())
}

#[allow(dead_code)]
#[napi]
pub fn seq_set_napi_buf_size(env: Env, ee: JsObject, size: JsNumber) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  let size = size.get_uint32()?;
  wrap.set_read_buf_size(size);
  Ok(())
}

#[allow(dead_code)]
#[napi]
pub fn seq_start_recv(env: Env, ee: JsObject) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  wrap.start_recv()?;
  Ok(())
}

#[allow(dead_code)]
#[napi]
pub fn seq_address(env: Env, ee: JsObject) -> Result<JsString> {
  let wrap = unwrap(&env, &ee)?;
  wrap.address(env)
}

#[allow(dead_code)]
#[napi]
pub fn seq_listen(env: Env, ee: JsObject, bindpath: JsString, backlog: JsNumber) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  wrap.listen(bindpath, backlog)
}

#[allow(dead_code)]
#[napi]
pub fn seq_connect(env: Env, ee: JsObject, server_path: JsString) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  wrap.connect(server_path)
}

#[allow(dead_code)]
#[napi]
pub fn seq_write(
  env: Env,
  ee: JsObject,
  buf: JsBuffer,
  offset: JsNumber,
  length: JsNumber,
  cb: Option<JsFunction>,
) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  wrap.write(env, buf, offset, length, cb)
}

#[allow(dead_code)]
#[napi]
pub fn seq_close(env: Env, ee: JsObject) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  wrap.close()
}

#[allow(dead_code)]
#[napi]
pub fn seq_shutdown_when_flushed(env: Env, ee: JsObject) -> Result<()> {
  let wrap = unwrap(&env, &ee)?;
  wrap.shutdown_when_flushed()
}

impl SeqpacketSocketWrap {
  fn wrap_obj(env: Env, mut this: JsObject, fd: Option<JsNumber>) -> Result<()> {
    // TODO
    // let ty = libc::SOCK_SEQPACKET;
    let ty = libc::SOCK_STREAM;
    let domain = libc::AF_UNIX;
    let protocol = 0;
    let fd: i32 = match fd {
      Some(fd) => fd.get_int32()?,
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

    let emit_fn = this.get_named_property::<JsFunction>("emit")?;
    let emit_ref = env.create_reference(emit_fn)?;

    let handle = Box::into_raw(Box::new(unsafe {
      mem::MaybeUninit::<sys::uv_poll_t>::zeroed().assume_init()
    }));
    let uv_loop = get_loop(&env)?;
    resolve_uv_err(unsafe { sys::uv_poll_init(uv_loop, handle, fd) })?;

    let wrap = Self {
      // this,
      fd,
      emit_ref,
      env,
      handle,
      msg_queue: LinkedList::new(),
      read_buf_size: DEFAULT_READ_BUF_SIZE,
      state: State::NewSocket,
      poll_events: 0,
    };
    env.wrap(&mut this, wrap)?;

    let this_ref = env.create_reference(this)?;
    let handle_data = Box::into_raw(Box::new(HandleData { env, this_ref }));

    unsafe { (*handle).data = handle_data as *mut _ };

    Ok(())
  }

  fn close(&mut self) -> Result<()> {
    if self.state == State::Closed {
      return Ok(());
    }

    let env = self.env;
    // close handle
    self.stop_poll()?;

    unsafe {
      sys::uv_close(self.handle as *mut _, Some(on_close));
    };

    // release msg_queue
    loop {
      let msg = self.msg_queue.pop_front();
      if msg.is_none() {
        break;
      }

      let mut msg = msg.unwrap();
      if msg.cb.is_some() {
        let mut cb = msg.cb.take().unwrap();
        cb.unref(env)?;
      }
    }

    // release js objects
    self.emit_ref.unref(env)?;

    socket::close(self.fd)?;

    self.state = State::Closed;

    self.emit_event("close")?;

    Ok(())
  }

  fn shutdown_write(&mut self) -> Result<()> {
    resolve_libc_err(unsafe { libc::shutdown(self.fd, libc::SHUT_WR) })?;
    self.state = State::ShutDown;
    self.emit_event("_shutdown")?;
    Ok(())
  }

  fn emit_error(&mut self, error: napi::Error) {
    let env = self.env;
    self.stop_poll().unwrap();
    // TODO unwrap
    env
      .run_in_scope(|| {
        let event = env.create_string("_error").unwrap();
        let error = self.env.create_error(error).unwrap();
        self
          .emit(&[event.into_unknown(), error.into_unknown()])
          .unwrap();
        Ok(())
      })
      .unwrap();
  }

  fn emit_event(&mut self, event: &str) -> Result<()> {
    let env = self.env;
    env.run_in_scope(|| {
      let js_event = env.create_string(event)?;
      let mut args: Vec<JsUnknown> = vec![];
      args.push(js_event.into_unknown());

      self.emit(&args)
    })?;
    Ok(())
  }

  fn emit(&mut self, args: &[JsUnknown]) -> Result<()> {
    let env = self.env;

    env.run_in_scope(|| {
      let emit: JsFunction = env.get_reference_value(&self.emit_ref)?;
      emit.call(None, args)?;
      Ok(())
    })?;

    Ok(())
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

  fn handle_connect(&mut self, status: i32, _events: i32) {
    if !self.check_uv_status(status) {
      return;
    }

    // FIXME: error ignored
    let _ = self.emit_event("_connect");
  }

  fn handle_socket(&mut self, status: i32, _events: i32) {
    if !self.check_uv_status(status) {
      return;
    }
    let mut addr = unsafe { mem::MaybeUninit::<sockaddr_un>::zeroed().assume_init() };
    let mut addr_len = mem::size_of::<sockaddr_un>() as u32;
    let fd = match resolve_libc_err(unsafe {
      libc::accept(
        self.fd,
        &mut addr as *mut _ as *mut libc::sockaddr,
        &mut addr_len as *mut _,
      )
    }) {
      Ok(fd) => fd,
      Err(e) => {
        self.emit_error(e);
        return;
      }
    };

    let env = self.env;
    let addr = addr_to_string(&addr);

    match env.run_in_scope(|| {
      let mut args: Vec<JsUnknown> = vec![];
      let js_event = env.create_string("_connection")?;
      args.push(js_event.into_unknown());
      let js_fd = env.create_int32(fd)?;
      args.push(js_fd.into_unknown());
      let js_addr = env.create_string(&addr)?;
      args.push(js_addr.into_unknown());
      self.emit(&args)?;
      Ok(())
    }) {
      Ok(_) => {}
      Err(e) => {
        let _ = env.throw_error(&e.reason, None);
      }
    }
  }

  fn handle_io(&mut self, status: i32, events: i32) {
    if !self.check_uv_status(status) {
      return;
    }

    if events & sys::uv_poll_event::UV_WRITABLE as i32 != 0 {
      self.flush();
    }

    if events & sys::uv_poll_event::UV_READABLE as i32 != 0 {
      match self._handle_readable() {
        Ok(_) => {}
        Err(e) => {
          self.emit_error(e);
        }
      };
    }
  }

  fn finish_msg(&self, mut msg: MsgItem) -> Result<()> {
    let env = self.env;

    if msg.cb.is_none() {
      return Ok(());
    }

    let mut cb = msg.cb.take().unwrap();

    let _ = env.run_in_scope(|| {
      let args: Vec<JsUnknown> = vec![];
      let cb: JsFunction = env.get_reference_value(&cb)?;
      let _ = cb.call(None, &args).map_err(|e| {
        let _ = self.env.throw_error(&e.reason, None);
      });

      Ok(())
    });

    cb.unref(env)?;
    Ok(())
  }

  fn flush(&mut self) {
    match self._flush() {
      Ok(_) => {}
      Err(e) => {
        self.emit_error(e);
      }
    }
  }

  fn _flush(&mut self) -> Result<()> {
    let mut finished_msgs: LinkedList<MsgItem> = LinkedList::new();

    loop {
      let msg = self.msg_queue.pop_front();
      if msg.is_none() {
        break;
      }

      let mut msg = msg.unwrap();
      let size = msg.msg.len();
      let msg_ptr = msg.msg.as_ptr();

      let mut ret: i32;
      loop {
        ret = unsafe { libc::write(self.fd, msg_ptr as *const _, size) } as i32;

        if !(ret == -1 && errno() == libc::EINTR) {
          break;
        }
      }

      if ret >= 0 {
        if ret == (size as i32) {
          finished_msgs.push_front(msg);
        } else {
          msg.msg = msg.msg[(ret as usize)..].to_owned();
          self.msg_queue.push_front(msg);
          break;
        }
      } else {
        self.msg_queue.push_front(msg);

        let err: i32 = errno();
        if err == EAGAIN || err == EWOULDBLOCK || err == ENOBUFS {
          break;
        } else {
          resolve_libc_err(ret)?;
        }
      }
    }

    if self.msg_queue.len() > 0 {
      self.poll_events |= sys::uv_poll_event::UV_WRITABLE as i32;
      self.reset_poll()?;
    } else {
      if self.state == State::ShuttingDown {
        self.shutdown_write()?;
      } else {
        self.poll_events ^= sys::uv_poll_event::UV_WRITABLE as i32;
        self.reset_poll()?;
      }
    }

    loop {
      let msg = finished_msgs.pop_front();
      if msg.is_none() {
        break;
      }
      let msg = msg.unwrap();
      self.finish_msg(msg)?;
    }

    Ok(())
  }

  fn _handle_readable(&mut self) -> Result<()> {
    loop {
      let buf_len = self.read_buf_size;
      let mut buf: Vec<u8> = vec![0; buf_len];
      let buf_ptr = buf.as_mut_ptr();
      let mut ret: i32;
      loop {
        ret = unsafe { libc::read(self.fd, buf_ptr as *mut _, buf_len) } as i32;

        if !(ret < 0 && errno() == EINTR) {
          break;
        }
      }

      if ret < 0 {
        let err = errno();
        if err == EAGAIN || err == EWOULDBLOCK {
          self.poll_events |= sys::uv_poll_event::UV_READABLE as i32;
          self.reset_poll()?;
          break;
        } else {
          resolve_uv_err(ret)?;
          break;
        }
      } else {
        let size = ret as usize;

        let env = self.env;
        env.run_in_scope(|| {
          let mut args: Vec<JsUnknown> = vec![];
          let js_event = env.create_string("_data")?;
          args.push(js_event.into_unknown());
          let js_buf = env.create_buffer_with_data(buf[0..size].to_vec())?;
          args.push(js_buf.into_unknown());
          self.emit(&args)?;
          Ok(())
        })?;

        // stop recv if the buf size is zero
        if size == 0 {
          self.poll_events ^= sys::uv_poll_event::UV_READABLE as i32;
          self.reset_poll()?;
          break;
        }
      }
    }

    Ok(())
  }

  fn reset_poll(&mut self) -> Result<()> {
    let events = self.poll_events;
    let is_closing = unsafe { sys::uv_is_closing(self.handle as *mut _) } != 0;

    if is_closing {
      return Ok(());
    }

    // stop poll
    if events == 0 {
      resolve_uv_err(unsafe { sys::uv_poll_stop(self.handle) })?;
      return Ok(());
    }

    resolve_uv_err(unsafe { sys::uv_poll_start(self.handle, events, Some(on_io)) })?;

    Ok(())
  }

  fn stop_poll(&mut self) -> Result<()> {
    self.poll_events = 0;

    self.reset_poll()?;
    Ok(())
  }

  fn check_uv_status(&mut self, status: i32) -> bool {
    if status < 0 {
      let msg = uv_err_msg(status);
      let err = error(format!("uv callback of failed with error: {}", &msg));
      self.emit_error(err);
      return false;
    }

    return true;
  }

  fn set_read_buf_size(&mut self, size: u32) {
    self.read_buf_size = size as usize;
  }

  fn start_recv(&mut self) -> Result<()> {
    self.poll_events |= sys::uv_poll_event::UV_READABLE as i32;
    self.reset_poll()?;
    Ok(())
  }

  fn address(&self, env: Env) -> Result<JsString> {
    let str = socket_addr_to_string(self.fd)?;
    env.create_string(&str)
  }

  fn listen(&self, bindpath: JsString, backlog: JsNumber) -> Result<()> {
    // Should never call listen() with a fd for multiple times.
    let bindpath = bindpath.into_utf8()?;
    let backlog = backlog.get_int32()?;

    self.bind(bindpath.as_str()?)?;
    resolve_libc_err(unsafe { libc::listen(self.fd, backlog) })?;

    // poll UV_DISCONNECT?
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

  fn connect(&mut self, server_path: JsString) -> Result<()> {
    let server_path = server_path.into_utf8()?;
    let path = server_path.as_str()?;
    let (mut sockaddr, addr_len) = unsafe { sockaddr_from_string(path)? };

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

    unsafe {
      sys::uv_poll_start(
        self.handle,
        sys::uv_poll_event::UV_WRITABLE as i32,
        Some(on_connect),
      )
    };

    Ok(())
  }

  fn write(
    &mut self,
    env: Env,
    buf: JsBuffer,
    offset: JsNumber,
    length: JsNumber,
    cb: Option<JsFunction>,
  ) -> Result<()> {
    if self.state >= State::ShuttingDown {
      return Err(error("socket has been shutdown".to_string()));
    }
    let offset = offset.get_int32()?;
    let length = length.get_int32()?;
    let msg = buf_into_vec(buf, offset, length)?;
    self.msg_queue.push_back(MsgItem {
      msg,
      cb: match cb {
        Some(cb) => {
          let cb_ref = env.create_reference(cb)?;
          Some(cb_ref)
        },
        None => None,
      },
    });

    self.flush();

    Ok(())
  }

  fn shutdown_when_flushed(&mut self) -> Result<()> {
    self.state = State::ShuttingDown;

    if self.msg_queue.len() == 0 {
      self.shutdown_write()?;
    }
    // else shutdown when msgs flushed
    Ok(())
  }
}

extern "C" fn on_close(handle: *mut sys::uv_handle_t) {
  unsafe {
    let mut data = Box::from_raw((*handle).data as *mut HandleData);
    let env = data.env;
    let _ = data.this_ref.unref(env);
    Box::from_raw(handle);
  };
}

// TODO use macro to simplify these
extern "C" fn on_socket(handle: *mut sys::uv_poll_t, status: c_int, events: c_int) {
  if status == sys::uv_errno_t::UV_ECANCELED as i32 {
    return;
  }

  let data = unsafe { Box::from_raw((*handle).data as *mut HandleData) };
  let env = data.env;

  // TODO unwrap
  env
    .run_in_scope(|| {
      let this: JsObject = env.get_reference_value(&data.this_ref)?;
      let wrap = unwrap(&env, &this)?;

      wrap.handle_socket(status, events);
      Ok(())
    })
    .unwrap();
  Box::into_raw(data);
}

extern "C" fn on_connect(handle: *mut sys::uv_poll_t, status: c_int, events: c_int) {
  if status == sys::uv_errno_t::UV_ECANCELED as i32 {
    return;
  }

  let data = unsafe { Box::from_raw((*handle).data as *mut HandleData) };
  let env = data.env;
  // TODO unwrap
  env
    .run_in_scope(|| {
      let this: JsObject = env.get_reference_value(&data.this_ref)?;
      let wrap = unwrap(&env, &this)?;

      wrap.handle_connect(status, events);
      Ok(())
    })
    .unwrap();
  Box::into_raw(data);
}

extern "C" fn on_io(handle: *mut sys::uv_poll_t, status: c_int, events: c_int) {
  if status == sys::uv_errno_t::UV_ECANCELED as i32 {
    return;
  }

  let data = unsafe { Box::from_raw((*handle).data as *mut HandleData) };
  let env = data.env;
  // TODO unwrap
  env
    .run_in_scope(|| {
      let this: JsObject = env.get_reference_value(&data.this_ref)?;
      let wrap = unwrap(&env, &this)?;

      wrap.handle_io(status, events);
      Ok(())
    })
    .unwrap();
  Box::into_raw(data);
}
