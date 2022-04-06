// https://github.com/extendr/extendr/issues/248
// Caused by extendr_module!{} but updating doesn't seen to fix it.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::{ffi::CStr, net::TcpStream};

use bindingsr::*;
use commonr::{
    data::{SexprecHeader, Sxpinfo},
    net,
};
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
    // Safety: should be ok to alternate between using the reference and the pointer since neither is mutable.
    // I couldn't get MIRI to complain when testing even more questionable things like read-only accesses
    // through mutable references and pointers.
    let sexr = unsafe { &*sexp };

    let ty = unsafe { TYPEOF(sexp) };
    let sxpinfo = Sxpinfo {
        ty,
        scalar: unsafe { IS_SCALAR(sexp, ty) },
        obj: unsafe { OBJECT(sexp) },
        alt: unsafe { ALTREP(sexp) },
        gp: unsafe { LEVELS(sexp) }, // TODO difference from ENVFLAGS?
        mark: unsafe { MARK(sexp) },
        debug: unsafe { RDEBUG(sexp) },
        trace: unsafe { RTRACE(sexp) },
        spare: unsafe { RSTEP(sexp) },
        gcgen: sexr.sxpinfo.gcgen(), // NODE_GENERATION(s) is in memory.c so not available to us
        gccls: sexr.sxpinfo.gccls(), // NODE_CLASS(s) is in memory.c so not available to us
        named: unsafe { NAMED(sexp) },
        extra: sexr.sxpinfo.extra(), // Using BNDCELL_TAG causes an error when loading the .so
    };

    assert!(sxpinfo.ty >= 0);
    // LATER Rf_sexptype2char / sexptype2char? (returns the name in CAPS like inspect)
    let ty_cstr = unsafe { CStr::from_ptr(Rf_type2char(sxpinfo.ty as u32)) };
    let ty_name = ty_cstr.to_str().unwrap().to_owned();
    let s = format!("test {:?} {} {:?}", sexp, sxpinfo.ty, ty_name);
    rprintln!("sending to visualizr: {}", s);

    let sxpinfo_bits = sexr.sxpinfo._bitfield_1.get(0, 64);
    let msg = SexprecHeader {
        address: sexp as u64,
        ty_name,
        sxpinfo,
        sxpinfo_bits,
        attrib: sexr.attrib as u64,
        gengc_next_node: sexr.gengc_next_node as u64,
        gengc_prev_node: sexr.gengc_prev_node as u64,
    };

    // Open a new connection each time because I don't wanna deal with weirdness
    // like what happens if I store it in a thread local and then the lib gets updated and reloaded.
    let mut stream = TcpStream::connect("127.0.0.1:26000").unwrap();
    //stream.set_nodelay(true).unwrap();
    //stream.set_nonblocking(true).unwrap();

    let netmsg = net::serialize(msg);
    net::send(&netmsg, &mut stream).unwrap();
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
