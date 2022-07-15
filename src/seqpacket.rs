use napi::{Env, JsBuffer, JsFunction, JsObject, JsUnknown, Ref, Result};
use crate::socket::{close};
use crate::util::{get_err, set_non_block, set_clo_exec};

#[napi]
pub struct SeqpacketSocketWrap {
  fd: i32,
  env: Env,
}

#[napi]
impl SeqpacketSocketWrap {
  #[napi(constructor)]
  pub fn new(env: Env) -> Result<Self> {
    let domain = libc::AF_UNIX;
    let ty = libc::SOCK_SEQPACKET;
    let protocol = 0;

    let fd = unsafe { libc::socket(domain, ty, protocol) };
    if fd == -1 {
      return Err(get_err());
    }

    set_non_block(fd)?;
    set_clo_exec(fd)?;

    Ok(Self {
      fd,
      env,
    })
  }

  #[napi]
  pub fn write(&self, env: Env, buf: JsBuffer, cb: JsFunction) -> Result<()> {
    // TODO
    Ok(())
  }

  #[napi]
  pub fn close(&self, env: Env) -> Result<()> {
    // TODO
    close(self.fd)
  }
}
