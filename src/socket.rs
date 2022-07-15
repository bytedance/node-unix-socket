use std::mem;
use std::{str::FromStr};

use libc::{sockaddr_un};
use napi::{Result, Env, Ref, JsFunction, JsUnknown, JsObject};
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

#[cfg(target_os = "macos")]
fn sun_family() -> u8 {
  libc::AF_UNIX as u8
}

#[cfg(target_os = "linux")]
fn sun_family() -> u16 {
  libc::AF_UNIX as u16
}

pub(crate) fn sockaddr_from_string(bytes: &str) -> Result<(sockaddr_un, usize)> {
  let mut bytes = String::from_str(bytes).unwrap();
  bytes.push('\0');

  let mut sockaddr = unsafe { mem::MaybeUninit::<sockaddr_un>::zeroed().assume_init() };
  // looks like `sun_len` is not necessary
  // (*sockaddr).sun_len = bytes.len() as u8;
  sockaddr.sun_family = sun_family();

  let size = mem::size_of_val(&sockaddr.sun_path);

  if bytes.len() > size {
    return Err(error("path to bind is too long".to_string()));
  }
  let path = (&mut sockaddr.sun_path.as_mut_slice()[0..bytes.len()]) as *mut _ as *mut [u8];
  let path = unsafe { &mut *path };
  path.clone_from_slice(bytes.as_bytes());

  Ok((sockaddr, mem::size_of::<sockaddr_un>()))
}


pub(crate) struct Emitter {
  env: Env,
  emit_ref: Option<Ref<()>>,
}

impl Drop for Emitter {
  fn drop(&mut self) {
    self.unref().unwrap();
  }
}

impl Emitter {
  pub fn new(env: Env, emit: JsFunction) -> Result<Self> {
    let emit_ref = env.create_reference(emit)?;

    Ok(Self {
      env,
      emit_ref: Some(emit_ref),
    })
  }

  pub fn unref(&mut self) -> Result<()> {
    let mut emit_ref = self.emit_ref.take();

    match emit_ref.as_mut() {
      None => (),
      Some(emit_ref) => {
        emit_ref.unref(self.env)?;
      }
    }

    Ok(())
  }

  fn check_ref(&self) -> Result<()> {
    if self.emit_ref.is_none() {
      return Err(error("emitter already unreferenced".to_string()));
    }

    Ok(())
  }

  pub fn emit(&mut self, args: &[JsUnknown]) -> Result<()> {
    self.check_ref()?;

    let env = self.env;

    env.run_in_scope(|| {
      let emit_ref = self.emit_ref.as_mut().unwrap();
      let emit: JsFunction = env.get_reference_value(emit_ref)?;
      emit.call(None, args)?;
      Ok(())
    })?;

    Ok(())
  }

  pub fn emit_event(&mut self, event: &str) -> Result<()> {
    let env = self.env;
    env.run_in_scope(|| {
      let js_event = env.create_string(event)?;
      let mut args: Vec<JsUnknown> = vec![];
      args.push(js_event.into_unknown());

      self.emit(&args)
    })?;
    Ok(())
  }
}


pub(crate) struct HandleData {
  env: Env,
  this_ref: Ref<()>,
}

impl HandleData {
  pub fn new(env: Env, this: JsObject) -> Result<Self> {
    let this_ref = env.create_reference(this)?;
    Ok(HandleData { env, this_ref })
  }

  pub fn clone_env(&self) -> Env {
    self.env
  }

  pub fn inner_mut_ref<'a, T: 'static>(&'a self) -> Result<&'a mut T> {
    let env = self.env;
    let native = env.run_in_scope(|| {
      let obj: JsObject = self.env.get_reference_value(&self.this_ref)?;
      let native: &mut T = self.env.unwrap(&obj)?;
      Ok(native)
    })?;
    Ok(native)
  }

  pub fn unref(&mut self) -> Result<()> {
    let env = self.env;
    self.this_ref.unref(env)?;
    Ok(())
  }
}
