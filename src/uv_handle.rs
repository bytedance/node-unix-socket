use std::{
  collections::HashSet,
  sync::{Mutex, MutexGuard, OnceLock},
};

use crate::util;
use napi::{Env, Result};
use uv_sys::sys::{uv_close, uv_handle_t};

// UV_HANDLES should only be used in clean up hooks
static mut UV_HANDLES: OnceLock<Mutex<HashSet<*mut uv_handle_t>>> = OnceLock::new();
static HOOK_INITED: OnceLock<bool> = OnceLock::new();

fn get_handles<T>(f: T) -> Result<()>
where
  T: FnOnce(MutexGuard<'_, HashSet<*mut uv_handle_t>>) -> Result<()>,
{
  let handles = unsafe { UV_HANDLES.get_or_init(|| Mutex::new(HashSet::new())) };

  {
    let inner = handles.lock();
    if inner.is_err() {
      let e = inner.err().unwrap();
      return Err(util::error(e));
    }
    let inner = inner.unwrap();
    f(inner)?
  }

  Ok(())
}

pub(crate) fn insert_handle(handle: *mut uv_handle_t) -> Result<()> {
  get_handles(|mut inner| {
    inner.insert(handle);
    Ok(())
  })
}

// TODO pointer same?
pub(crate) fn remove_handle(handle: *mut uv_handle_t) -> Result<()> {
  get_handles(|mut inner| {
    inner.remove(&handle);
    Ok(())
  })
}

pub(crate) fn cleanup_handles() -> Result<()> {
  let handles = unsafe { UV_HANDLES.get() };
  if handles.is_none() {
    return Ok(());
  }

  get_handles(|mut inner| {
    for handle in inner.drain() {
      if handle.is_null() {
        continue;
      }
      unsafe {
        uv_close(handle, None);
      };
    }

    Ok(())
  })
}

#[napi]
#[allow(dead_code)]
pub fn init_cleanup_hook(mut env: Env) -> Result<()> {
  if HOOK_INITED.get().is_some() {
    return Ok(());
  }
  HOOK_INITED.get_or_init(|| true);

  env.add_env_cleanup_hook((), move |_| {
    let result = cleanup_handles();
    if result.is_err() {
      println!(
        "cleanup_handles failed, msg: {}",
        result.err().unwrap().to_string()
      )
    }
  })?;

  Ok(())
}
