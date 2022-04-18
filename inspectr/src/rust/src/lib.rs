// https://github.com/extendr/extendr/issues/248
// Caused by extendr_module!{} but updating doesn't seen to fix it.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::{collections::HashSet, ffi::CStr, net::TcpStream};

use bindingsr::*;
use commonr::{data::*, net};
use extendr_api::prelude::*;

/*
For testing:

0 NILSXP    visualize(NULL)
1 SYMSXP    x <- 1; visualize(substitute(x))
2 LISTSXP   visualize(pairlist(1,2,3))
3 CLOSXP    visualize(visualize)
4 ENVSXP
5 PROMSXP
6 LANGSXP   visualize(substitute(2+2))
7 SPECIALSXP
8 BUILTINSXP    visualize(`(`) ; visualize(`-`)
9 CHARSXP
    contained in STRSXP
10 LGLSXP
13 INTSXP
14 REALSXP  visualize(1)
15 CPLXSXP
16 STRSXP   visualize("a")
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

// TODO how to receive cmds????
//     1) loop
//     2) thread from Rust (R is single threaded)
//     3) pause process?

/// Inspect obj's representation using visualizr.
/// @export
#[extendr]
fn visualize(obj: Robj) {
    let globals = get_globals();

    let sexp = to_sexp(obj);

    let sexprecs = walk_sexps(sexp);

    let update = Update {
        globals,
        sexprecs,
    };
    rprintln!("{}", update);
    rprintln!("sending {} sexp(s)", update.sexprecs.len());

    // Open a new connection each time because I don't wanna deal with weirdness
    // like what happens if I store it in a thread local and then the lib gets updated and reloaded.
    let mut stream = TcpStream::connect("127.0.0.1:26000").unwrap();
    let netmsg = net::serialize(update);
    net::send(&netmsg, &mut stream).unwrap();
}

fn walk_sexps(sexp: SEXP) -> Vec<Sexprec> {
    let mut walker = Walker {
        visited: HashSet::new(),
        sexprecs: Vec::new(),
    };
    walker.walk_sexp(sexp);
    walker.sexprecs
}

#[derive(Debug)]
struct Walker {
    visited: HashSet<SEXP>,
    sexprecs: Vec<Sexprec>,
}

impl Walker {
    fn walk_sexp(&mut self, sexp: SEXP) {
        if self.visited.contains(&sexp) {
            return;
        }
        self.visited.insert(sexp);

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
        let sxpinfo_bits = sexr.sxpinfo._bitfield_1.get(0, 64);

        let ty = Sexptype::try_from(ty_int).unwrap();

        let (payload, ptrs) = match ty {
            Sexptype::SYMSXP => get_symsxp_payload(sexr),
            Sexptype::LISTSXP | Sexptype::LANGSXP | Sexptype::EXPRSXP => get_listsxp_payload(sexr),
            Sexptype::CLOSXP  => get_closxp_payload(sexr),
            Sexptype::ENVSXP => get_envsxp_payload(sexr),
            Sexptype::PROMSXP => get_promsxp_payload(sexr),
            Sexptype::SPECIALSXP | Sexptype::BUILTINSXP=>get_primsxp_payload(sexr),
            Sexptype::CHARSXP
            | Sexptype::LGLSXP
            | Sexptype::INTSXP
            | Sexptype::REALSXP
            | Sexptype::CPLXSXP
            | Sexptype::STRSXP
            | Sexptype::VECSXP
            | Sexptype::RAWSXP => get_vecsxp(sexp),
            Sexptype::NILSXP // Explicitly initialized as list in memory.c
            | Sexptype::DOTSXP
            | Sexptype::ANYSXP
            | Sexptype::BCODESXP
            | Sexptype::EXTPTRSXP
            | Sexptype::WEAKREFSXP
            | Sexptype::S4SXP
            | Sexptype::NEWSXP
            | Sexptype::FREESXP => get_default_payload(sexr),
        };

        // unsafe {
        //     dbg!(sexr.u.listsxp.carval);
        //     dbg!(sexr.u.listsxp.cdrval);
        //     dbg!(sexr.u.listsxp.tagval);
        //     dbg!(*std::ptr::addr_of!(sexr.u.listsxp.carval).offset(0));
        //     dbg!(*std::ptr::addr_of!(sexr.u.listsxp.carval).offset(1));
        //     dbg!(*std::ptr::addr_of!(sexr.u.listsxp.carval).offset(2));
        //     dbg!(*std::ptr::addr_of!(sexr.u.listsxp.carval).offset(4));
        //     dbg!(*std::ptr::addr_of!(sexr.u.listsxp.carval).offset(5));
        // }

        // LATER Rf_sexptype2char / sexptype2char? (returns the name in CAPS like inspect)
        let ty_cstr = unsafe { CStr::from_ptr(Rf_type2char(sxpinfo.ty as u32)) };
        let ty_name = ty_cstr.to_str().unwrap().to_owned();

        let sexprec = Sexprec {
            address: sexp.into(),
            ty,
            ty_name,
            sxpinfo,
            sxpinfo_bits,
            attrib: sexr.attrib.into(),
            gengc_next_node: sexr.gengc_next_node.into(),
            gengc_prev_node: sexr.gengc_prev_node.into(),
            payload,
        };
        self.sexprecs.push(sexprec);

        self.walk_sexp(sexr.attrib);
        for ptr in ptrs {
            self.walk_sexp(ptr);
        }
    }
}

fn to_sexp(obj: Robj) -> SEXP {
    // Note the cast is from from libR_sys::SEXP to bindingsr::SEXP
    match obj {
        Robj::Owned(sexp) => sexp as bindingsr::SEXP,
        Robj::Sys(sexp) => sexp as bindingsr::SEXP,
    }
}

fn get_globals() -> Globals {
    // TODO use these for testing (also some stuff in bindings below them)
    // LATER more values? good list in memory.c - ctrl+f /* forward all roots */
    Globals {
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

fn get_vecsxp(sexp: *mut SEXPREC) -> (SexpPayload, Vec<SEXP>) {
    let sexp_align = sexp as *mut SEXPREC_ALIGN;
    let sexr_align = unsafe { &*sexp_align };

    // TODO This points to another sexp. Also more sexp pointers after that?

    let sxp = unsafe { &sexr_align.s.vecsxp };
    let payload = SexpPayload::Vecsxp(Vecsxp {
        length: sxp.length as i64,
        truelength: sxp.truelength as i64,
    });
    (payload, vec![])
}

fn get_primsxp_payload(sexr: &SEXPREC) -> (SexpPayload, Vec<SEXP>) {
    let sxp = unsafe { &sexr.u.primsxp };
    let payload = SexpPayload::Primsxp(Primsxp { offset: sxp.offset });
    (payload, vec![])
}

fn get_symsxp_payload(sexr: &SEXPREC) -> (SexpPayload, Vec<SEXP>) {
    let sxp = unsafe { sexr.u.symsxp };
    let ptrs = vec![sxp.pname, sxp.value, sxp.internal];
    let payload = SexpPayload::Symsxp(Symsxp {
        pname: sxp.pname.into(),
        value: sxp.value.into(),
        internal: sxp.internal.into(),
    });
    (payload, ptrs)
}

fn get_listsxp_payload(sexr: &SEXPREC) -> (SexpPayload, Vec<SEXP>) {
    let sxp = unsafe { sexr.u.listsxp };
    let ptrs = vec![sxp.carval, sxp.cdrval, sxp.tagval];
    let payload = SexpPayload::Listsxp(Listsxp {
        carval: sxp.carval.into(),
        cdrval: sxp.cdrval.into(),
        tagval: sxp.tagval.into(),
    });
    (payload, ptrs)
}

fn get_envsxp_payload(sexr: &SEXPREC) -> (SexpPayload, Vec<SEXP>) {
    let sxp = unsafe { sexr.u.envsxp };
    let ptrs = vec![sxp.frame, sxp.enclos, sxp.hashtab];
    let payload = SexpPayload::Envsxp(Envsxp {
        frame: sxp.frame.into(),
        enclos: sxp.enclos.into(),
        hashtab: sxp.hashtab.into(),
    });
    (payload, ptrs)
}

fn get_closxp_payload(sexr: &SEXPREC) -> (SexpPayload, Vec<SEXP>) {
    let sxp = unsafe { sexr.u.closxp };
    let ptrs = vec![sxp.formals, sxp.body, sxp.env];
    let payload = SexpPayload::Closxp(Closxp {
        formals: sxp.formals.into(),
        body: sxp.body.into(),
        env: sxp.env.into(),
    });
    (payload, ptrs)
}

fn get_promsxp_payload(sexr: &SEXPREC) -> (SexpPayload, Vec<SEXP>) {
    let sxp = unsafe { sexr.u.promsxp };
    let ptrs = vec![sxp.value, sxp.expr, sxp.env];
    let payload = SexpPayload::Promsxp(Promsxp {
        value: sxp.value.into(),
        expr: sxp.expr.into(),
        env: sxp.env.into(),
    });
    (payload, ptrs)
}

fn get_default_payload(sexr: &SEXPREC) -> (SexpPayload, Vec<SEXP>) {
    let sxp = unsafe { sexr.u.listsxp };
    let ptrs = vec![sxp.carval, sxp.cdrval, sxp.tagval];
    let payload = SexpPayload::Listsxp(Listsxp {
        carval: sxp.carval.into(),
        cdrval: sxp.cdrval.into(),
        tagval: sxp.tagval.into(),
    });
    (payload, ptrs)
}

// Macro to generate exports.
// This ensures exported functions are registered with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod inspectr;
    fn visualize;
}
