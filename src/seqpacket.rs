use napi::{Env, JsBuffer, JsFunction, JsObject, JsUnknown, Ref, Result};

#[napi]
struct SeqpacketSocketWrap {}

#[napi]
impl SeqpacketSocketWrap {
  #[napi(constructor)]
  pub fn new() -> Result<Self> {
    Ok(Self {})
  }
}
