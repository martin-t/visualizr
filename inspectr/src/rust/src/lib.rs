// https://github.com/extendr/extendr/issues/248
// Caused by extendr_module!{} but updating doesn't seen to fix it.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::{ffi::CStr, net::TcpStream};

use bindingsr::*;
use commonr::net;
use extendr_api::prelude::*;

// For testing:
/*
rextendr::document() ; devtools::load_all()

s <- substitute(2+2)
.Internal(inspect( s ))

visualize(s)
 */

/// Inspect obj's representation using visualizr.
/// @export
#[extendr]
fn visualize(obj: Robj) {
    let sexp = to_sexp(obj);
    //println!("test {:?}", sexp); // LATER lint against normal print(ln)?
    let ty = unsafe { TYPEOF(sexp) };
    assert!(ty >= 0);
    // TODO safety
    // TODO Rf_sexptype2char / sexptype2char
    let ty_name = unsafe { CStr::from_ptr(Rf_type2char(ty as u32)) };
    let s = format!("test {:?} {} {:?}", sexp, ty, ty_name);
    rprintln!("sending to visualizr: {}", s);

    // Open a new connection each time because I don't wnna deal with weirdness
    // like what happens if I store it in a thread local and then the lib gets updated and reloaded.
    let mut stream = TcpStream::connect("127.0.0.1:26000").unwrap();
    //stream.set_nodelay(true).unwrap();
    //stream.set_nonblocking(true).unwrap();

    let msg = net::serialize(s);
    net::send(&msg, &mut stream).unwrap();
}

fn to_sexp(obj: Robj) -> SEXP {
    // Note the cast is from from libR_sys::SEXP to bindingsr::SEXP
    match obj {
        Robj::Owned(sexp) => sexp as SEXP,
        Robj::Sys(sexp) => sexp as SEXP,
    }
}

// Macro to generate exports.
// This ensures exported functions are registered with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod inspectr;
    fn visualize;
}
