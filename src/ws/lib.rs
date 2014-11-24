//#![crate_type = "dylib"] // FIXME this fails with regular rust-crypto, since just builds rlibs by default. TODO PR to fix that
#![crate_type = "rlib"]

#![feature(phase)]
#[phase(plugin, link)] extern crate log;

extern crate time;
extern crate serialize;
extern crate http;
extern crate "rust-crypto" as rust_crypto;

pub mod server;
pub mod message;
