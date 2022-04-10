// https://github.com/extendr/extendr/issues/248
// Caused by extendr_module!{} but updating doesn't seen to fix it.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::{ffi::CStr, net::TcpStream};

use bindingsr::*;
use commonr::{data::*, net};
use extendr_api::prelude::*;

/*
For testing:


0 NILSXP visualize(NULL)
1 SYMSXP
2 LISTSXP
3 CLOSXP visualize(visualize)
4 ENVSXP
5 PROMSXP
6 LANGSXP visualize(substitute(2+2))
7 SPECIALSXP
8 BUILTINSXP visualize(`(`)
9 CHARSXP
10 LGLSXP
13 INTSXP
14 REALSXP visualize(1)
15 CPLXSXP
16 STRSXP visualize("a")
17 DOTSXP
18 ANYSXP
19 VECSXP
20 EXPRSXP
21 BCODESXP
22 EXTPTRSXP
23 WEAKREFSXP
24 RAWSXP
25 S4SXP
30 NEWSXP
31 FREESXP
*/

/// Inspect obj's representation using visualizr.
/// @export
#[extendr]
fn visualize(obj: Robj) {
    let special_values = get_special_values();

    let sexp = to_sexp(obj);
    // Safety: should be ok to alternate between using the reference and the pointer.
    // I couldn't get MIRI to complain when testing even more questionable things like read-only accesses
    // through mutable references.
    let sexr = unsafe { &*sexp };

    let ty_int = unsafe { TYPEOF(sexp) };
    let sxpinfo = Sxpinfo {
        ty: ty_int,
        scalar: unsafe { IS_SCALAR(sexp, ty_int) },
        obj: unsafe { OBJECT(sexp) },
        alt: unsafe { ALTREP(sexp) },
        gp: unsafe { LEVELS(sexp) },
        mark: unsafe { MARK(sexp) },
        debug: unsafe { RDEBUG(sexp) },
        trace: unsafe { RTRACE(sexp) },
        spare: unsafe { RSTEP(sexp) },
        gcgen: sexr.sxpinfo.gcgen(), // NODE_GENERATION(s) is in memory.c so not available to us
        gccls: sexr.sxpinfo.gccls(), // NODE_CLASS(s) is in memory.c so not available to us
        named: unsafe { NAMED(sexp) },
        extra: sexr.sxpinfo.extra(), // Using BNDCELL_TAG causes an error when loading the .so
    };
    // TODO named special meaning? NULL has 65535

    // TODO GP - from inspect.c
    // if (IS_S4_OBJECT(v)) { if (a) Rprintf(","); Rprintf("S4"); a = 1; }
    // if (TYPEOF(v) == SYMSXP || TYPEOF(v) == LISTSXP) {
    //     if (IS_ACTIVE_BINDING(v)) { if (a) Rprintf(","); Rprintf("AB"); a = 1; }
    //     if (BINDING_IS_LOCKED(v)) { if (a) Rprintf(","); Rprintf("LCK"); a = 1; }
    // }
    // if (TYPEOF(v) == ENVSXP) {
    //     if (FRAME_IS_LOCKED(v)) { if (a) Rprintf(","); Rprintf("LCK"); a = 1; }
    //     if (IS_GLOBAL_FRAME(v)) { if (a) Rprintf(","); Rprintf("GL"); a = 1; }
    // }

    // TODO recurse into attrib if not nil

    // TODO
    // if (ALTREP(v) && ALTREP_INSPECT(v, pre, deep, pvec, inspect_subtree)) {
    // if (ATTRIB(v) && ATTRIB(v) != R_NilValue && TYPEOF(v) != CHARSXP) {
    //     pp(pre);
    //     Rprintf("ATTRIB:\n");
    //     inspect_tree(pre+2, ATTRIB(v), deep, pvec);
    // }
    // return;
    // }

    let ty = Sexptype::try_from(ty_int).unwrap();

    // TODO inspect.c line 130-194 - print more type-specific info
    // Note we must not alter the internal state - careful when printing.

    let payload = match ty {
        Sexptype::NILSXP => SexpPayload::Nothing,
        Sexptype::SYMSXP => get_symsxp(sexr),
        Sexptype::LISTSXP | Sexptype::LANGSXP | Sexptype::EXPRSXP => get_listsxp(sexr),
        Sexptype::CLOSXP | Sexptype::SPECIALSXP | Sexptype::BUILTINSXP => get_closxp(sexr),
        Sexptype::ENVSXP => get_envsxp(sexr),
        Sexptype::PROMSXP => get_promsxp(sexr),
        Sexptype::CHARSXP
        | Sexptype::LGLSXP
        | Sexptype::INTSXP
        | Sexptype::REALSXP
        | Sexptype::CPLXSXP
        | Sexptype::STRSXP
        | Sexptype::VECSXP
        | Sexptype::RAWSXP => get_vecsxp(sexp),
        Sexptype::DOTSXP
        | Sexptype::ANYSXP
        | Sexptype::BCODESXP
        | Sexptype::EXTPTRSXP
        | Sexptype::WEAKREFSXP
        | Sexptype::S4SXP
        | Sexptype::NEWSXP
        | Sexptype::FREESXP => get_default_sxp(sexr),
    };

    unsafe {
        dbg!(sexr.u.listsxp.carval);
        dbg!(sexr.u.listsxp.cdrval);
        dbg!(sexr.u.listsxp.tagval);
        dbg!(*std::ptr::addr_of!(sexr.u.listsxp.carval).offset(0));
        dbg!(*std::ptr::addr_of!(sexr.u.listsxp.carval).offset(1));
        dbg!(*std::ptr::addr_of!(sexr.u.listsxp.carval).offset(2));
        dbg!(*std::ptr::addr_of!(sexr.u.listsxp.carval).offset(4));
        dbg!(*std::ptr::addr_of!(sexr.u.listsxp.carval).offset(5));
    }

    // LATER Rf_sexptype2char / sexptype2char? (returns the name in CAPS like inspect)
    let ty_cstr = unsafe { CStr::from_ptr(Rf_type2char(sxpinfo.ty as u32)) };
    let ty_name = ty_cstr.to_str().unwrap().to_owned();
    let sxpinfo_bits = sexr.sxpinfo._bitfield_1.get(0, 64);
    let nil = unsafe { R_NilValue };

    let sexprec = Sexprec {
        address: sexp.into(),
        ty,
        ty_name,
        sxpinfo,
        sxpinfo_bits,
        attrib: sexr.attrib.into(),
        attrib_nil: sexr.attrib == nil,
        gengc_next_node: sexr.gengc_next_node.into(),
        gengc_prev_node: sexr.gengc_prev_node.into(),
        payload,
    };

    let update = Update {
        special_values,
        sexprec,
    };
    rprintln!("{}", update);

    // Open a new connection each time because I don't wanna deal with weirdness
    // like what happens if I store it in a thread local and then the lib gets updated and reloaded.
    let mut stream = TcpStream::connect("127.0.0.1:26000").unwrap();
    //stream.set_nodelay(true).unwrap();
    //stream.set_nonblocking(true).unwrap();

    let netmsg = net::serialize(update);
    net::send(&netmsg, &mut stream).unwrap();
}

fn to_sexp(obj: Robj) -> SEXP {
    // Note the cast is from from libR_sys::SEXP to bindingsr::SEXP
    match obj {
        Robj::Owned(sexp) => sexp as bindingsr::SEXP,
        Robj::Sys(sexp) => sexp as bindingsr::SEXP,
    }
}

fn get_special_values() -> SpecialValues {
    // TODO use these for testing (also some stuff in bindings below them)
    SpecialValues {
        unbound_value: unsafe { R_UnboundValue.into() },
        nil_value: unsafe { R_NilValue.into() },
        missing_arg: unsafe { R_MissingArg.into() },
        global_env: unsafe { R_GlobalEnv.into() },
        empty_env: unsafe { R_EmptyEnv.into() },
        base_env: unsafe { R_BaseEnv.into() },
        base_namespace: unsafe { R_BaseNamespace.into() },
        namespace_registry: unsafe { R_NamespaceRegistry.into() },
        src_ref: unsafe { R_Srcref.into() },
        in_bc_interpreter: unsafe { R_InBCInterpreter.into() },
        current_expression: unsafe { R_CurrentExpression.into() },
        // Using R_RestartToken causes an error when loading the .so
        //restart_token: unsafe { R_RestartToken.into() },
    }
}

fn get_vecsxp(sexp: *mut SEXPREC) -> SexpPayload {
    let sexp_align = sexp as *mut SEXPREC_ALIGN;
    let sexr_align = unsafe { &*sexp_align };

    SexpPayload::Vecsxp(Vecsxp {
        length: unsafe { sexr_align.s.vecsxp.length as i64 },
        truelength: unsafe { sexr_align.s.vecsxp.truelength as i64 },
    })
}

// TODO primsxp?

fn get_symsxp(sexr: &SEXPREC) -> SexpPayload {
    SexpPayload::Symsxp(Symsxp {
        pname: unsafe { sexr.u.symsxp.pname.into() },
        value: unsafe { sexr.u.symsxp.value.into() },
        internal: unsafe { sexr.u.symsxp.internal.into() },
    })
}

fn get_listsxp(sexr: &SEXPREC) -> SexpPayload {
    SexpPayload::Listsxp(Listsxp {
        carval: unsafe { sexr.u.listsxp.carval.into() },
        cdrval: unsafe { sexr.u.listsxp.cdrval.into() },
        tagval: unsafe { sexr.u.listsxp.tagval.into() },
    })
}

fn get_envsxp(sexr: &SEXPREC) -> SexpPayload {
    SexpPayload::Envsxp(Envsxp {
        frame: unsafe { sexr.u.envsxp.frame.into() },
        enclos: unsafe { sexr.u.envsxp.enclos.into() },
        hashtab: unsafe { sexr.u.envsxp.hashtab.into() },
    })
}

fn get_closxp(sexr: &SEXPREC) -> SexpPayload {
    SexpPayload::Closxp(Closxp {
        formals: unsafe { sexr.u.closxp.formals.into() },
        body: unsafe { sexr.u.closxp.body.into() },
        env: unsafe { sexr.u.closxp.env.into() },
    })
}

fn get_promsxp(sexr: &SEXPREC) -> SexpPayload {
    SexpPayload::Promsxp(Promsxp {
        value: unsafe { sexr.u.promsxp.value.into() },
        expr: unsafe { sexr.u.promsxp.expr.into() },
        env: unsafe { sexr.u.promsxp.env.into() },
    })
}

fn get_default_sxp(sexr: &SEXPREC) -> SexpPayload {
    SexpPayload::Listsxp(Listsxp {
        carval: unsafe { sexr.u.listsxp.carval.into() },
        cdrval: unsafe { sexr.u.listsxp.cdrval.into() },
        tagval: unsafe { sexr.u.listsxp.tagval.into() },
    })
}

// Macro to generate exports.
// This ensures exported functions are registered with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod inspectr;
    fn visualize;
}
