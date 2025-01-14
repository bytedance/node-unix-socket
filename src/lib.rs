#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

mod seqpacket;
mod dgram;
mod util;
mod socket;
mod uv_handle;
