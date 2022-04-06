#[allow(improper_ctypes)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[allow(deref_nullptr)] // https://github.com/rust-lang/rust-bindgen/issues/1651
#[allow(clippy::all)]
mod bindings;

pub use bindings::*;
