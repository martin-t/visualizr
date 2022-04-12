use std::fmt::{self, Display, Formatter};

use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Update {
    pub global_values: GlobalValues,
    pub sexprec: Sexprec,
}

#[rustfmt::skip]
impl Display for Update {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "address: {}, type: {}/{}",
            self.global_values.fmt_ptr(self.sexprec.address),
            self.sexprec.ty_name,
            self.sexprec.ty,
        )?;

        writeln!(f, "sxpinfo: {:#066b}", self.sexprec.sxpinfo_bits)?;
        // named and extra are 16 bits so 5 digits is exactly enough,
        // everything else has more space than needed
        writeln!(f, " fields: type scalar obj alt           gp          mark debug trace spare gcgen gccls named extra")?;
        writeln!(f, "   bits:  [5]    [1] [1] [1]          [16]         [1]   [1]   [1]   [1]   [1]   [3]  [16]  [16]")?;
        // GP needs 18 chars: 2 for "0b" and 16 for the bits
        writeln!(f, "         {:4} {:6} {:3} {:3}  {:#018b}  {:4} {:5} {:5} {:5} {:5} {:5} {:5} {:5}",
            self.sexprec.sxpinfo.ty,
            self.sexprec.sxpinfo.scalar,
            self.sexprec.sxpinfo.obj,
            self.sexprec.sxpinfo.alt,
            self.sexprec.sxpinfo.gp,
            self.sexprec.sxpinfo.mark,
            self.sexprec.sxpinfo.debug,
            self.sexprec.sxpinfo.trace,
            self.sexprec.sxpinfo.spare,
            self.sexprec.sxpinfo.gcgen,
            self.sexprec.sxpinfo.gccls,
            self.sexprec.sxpinfo.named,
            self.sexprec.sxpinfo.extra,
        )?;

        // GP:
        // 0: DDVAL / HASHASH / READY_TO_FINALIZE       |
        // 1: BYTES / FINALIZE_ON_EXIT                  | MISSING
        // 2: LATIN1                                    |
        // 3: UTF8                                      |
        // 4: S4
        // 5: NOJIT / GROWABLE / CACHED
        // 6: ASCII
        // 7:
        // 8:
        // 9:
        // 10:
        // 11: ASSIGNMENT_PENDING
        // 12: SPECIAL_SYMBOL / NO_SPECIAL_SYMBOLS
        // 13: BASE_SYM_CACHED
        // 14: BINDING_LOCK / FRAME_LOCK
        // 15: ACTIVE_BINDING / GLOBAL_FRAME

        // ENC_KNOWN = LATIN1_MASK | UTF8_MASK
        // ENVFLAGS / PRSEEN / LEVELS / ARGUSED / OLDTYPE = GP
        // PRSEEN: R-ints says only bit 0
        // GROWABLE is true only if (&& XLENGTH(x) < XTRUELENGTH(x))
        // Latin-1, UTF-8 or ASCII + cached - only CHARSXP according to R-ints
        // check inspect.c - some bits only apply for some types

        // TODO recheck all the bits and masks before release

        writeln!(f, "         GP bit meaning: |< MISSING [0:3] >|                                   ASSIGNMENT_PENDING")?;
        writeln!(f, "                         DDVAL / HASHASH / READY_TO_FINALIZE                   |    SPECIAL_SYMBOL / NO_SPECIAL_SYMBOLS")?;
        writeln!(f, "                         |    BYTES / FINALIZE_ON_EXIT                         |    |    BASE_SYM_CACHED")?;
        writeln!(f, "                         |    |    LATIN1         NOJIT / GROWABLE / CACHED    |    |    |    BINDING_LOCK / FRAME_LOCK")?;
        writeln!(f, "                         |    |    |    UTF8 S4   |    ASCII                   |    |    |    |    ACTIVE_BINDING / GLOBAL_FRAME")?;
        writeln!(f, "                  index: 0    1    2    3    4    5    6    7    8    9   10   11   12   13   14   15")?;
        write!(f, "                  value: ")?;
        for index in 0..16 {
            let bit = self.sexprec.sxpinfo.gp & (1 << index);
            let is_set = bit >> index; // Shift it so only 0 or 1 remains
            write!(f, "{:<5}", is_set)?;
        }
        writeln!(f)?;

        writeln!(f, "attrib {}", self.global_values.fmt_ptr(self.sexprec.attrib))?;
        writeln!(f, "gengc_next_node {}", self.global_values.fmt_ptr(self.sexprec.gengc_next_node))?;
        writeln!(f, "gengc_prev_node {}", self.global_values.fmt_ptr(self.sexprec.gengc_prev_node))?;

        match &self.sexprec.payload {
            SexpPayload::Nothing => writeln!(f, "no paylod")?, // TODO all have payload? - remove?
            SexpPayload::Vecsxp(vecsxp) => {
                writeln!(f, "length: {}", vecsxp.length)?;
                writeln!(f, "truelength: {}", vecsxp.truelength)?;
            }
            SexpPayload::Primsxp(primsxp) => {
                writeln!(f, "offset: {}", primsxp.offset)?;
            },
            SexpPayload::Symsxp(symsxp) => {
                writeln!(f, "pname: {}", self.global_values.fmt_ptr(symsxp.pname))?;
                writeln!(f, "value: {}", self.global_values.fmt_ptr(symsxp.value))?;
                writeln!(f, "internal: {}", self.global_values.fmt_ptr(symsxp.internal))?;
            },
            SexpPayload::Listsxp(listsxp) => {
                writeln!(f, "carval: {}", self.global_values.fmt_ptr(listsxp.carval))?;
                writeln!(f, "cdrval: {}", self.global_values.fmt_ptr(listsxp.cdrval))?;
                writeln!(f, "tagval: {}", self.global_values.fmt_ptr(listsxp.tagval))?;
            },
            SexpPayload::Envsxp(envsxp) => {
                writeln!(f, "frame: {}", self.global_values.fmt_ptr(envsxp.frame))?;
                writeln!(f, "enclos: {}", self.global_values.fmt_ptr(envsxp.enclos))?;
                writeln!(f, "hashtab: {}", self.global_values.fmt_ptr(envsxp.hashtab))?;
            },
            SexpPayload::Closxp(closxp) => {
                writeln!(f, "formals: {}", self.global_values.fmt_ptr(closxp.formals))?;
                writeln!(f, "body: {}", self.global_values.fmt_ptr(closxp.body))?;
                writeln!(f, "env: {}", self.global_values.fmt_ptr(closxp.env))?;
            },
            SexpPayload::Promsxp(promsxp) => {
                writeln!(f, "value: {}", self.global_values.fmt_ptr(promsxp.value))?;
                writeln!(f, "expr: {}", self.global_values.fmt_ptr(promsxp.expr))?;
                writeln!(f, "env: {}", self.global_values.fmt_ptr(promsxp.env))?;
            },
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobalValues {
    pub unbound_value: Sexp,
    pub nil_value: Sexp,
    pub missing_arg: Sexp,
    pub global_env: Sexp,
    pub empty_env: Sexp,
    pub base_env: Sexp,
    pub base_namespace: Sexp,
    pub namespace_registry: Sexp,
    pub src_ref: Sexp,
    pub in_bc_interpreter: Sexp,
    pub current_expression: Sexp,
    //pub restart_token: Sexp,
}

impl GlobalValues {
    #[must_use]
    fn fmt_ptr(&self, sexp: Sexp) -> String {
        // Don't impl Display for Sexp so it's impossible to accidentally forget to use this function.
        let mut s = format!("@{:x}", sexp.0);
        if sexp == self.unbound_value {
            s.push_str(" (R_UnboundValue)");
        } else if sexp == self.nil_value {
            s.push_str(" (R_NilValue)");
        } else if sexp == self.missing_arg {
            s.push_str(" (R_MissingArg)");
        } else if sexp == self.global_env {
            s.push_str(" (R_GlobalEnv)");
        } else if sexp == self.empty_env {
            s.push_str(" (R_EmptyEnv)");
        } else if sexp == self.base_env {
            s.push_str(" (R_BaseEnv)");
        } else if sexp == self.base_namespace {
            s.push_str(" (R_BaseNamespace)");
        } else if sexp == self.namespace_registry {
            s.push_str(" (R_NamespaceRegistry)");
        } else if sexp == self.src_ref {
            s.push_str(" (R_Srcref)");
        } else if sexp == self.in_bc_interpreter {
            s.push_str(" (R_InBCInterpreter)");
        } else if sexp == self.current_expression {
            s.push_str(" (R_CurrentExpression)");
        }
        // else if sexp == self.restart_token {
        //     s.push_str(" (R_RestartToken)");
        // }
        s
    }
}

// Serialize pointers as this because technically
// inspectr and visualizr could be running on different architectures.
// LATER What about integers? Need sufficient size for all architectures supported by R.
//      What about signed vs unsigned char?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct Sexp(pub u64);

impl<T> From<*mut T> for Sexp {
    fn from(ptr: *mut T) -> Self {
        Self(ptr as u64)
    }
}

#[derive(Debug, Deserialize, Serialize, IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub enum Sexptype {
    NILSXP = 0,
    SYMSXP = 1,
    LISTSXP = 2,
    #[num_enum(alternatives = [99])] // LATER does R ever return this?
    CLOSXP = 3,
    ENVSXP = 4,
    PROMSXP = 5,
    LANGSXP = 6,
    SPECIALSXP = 7,
    BUILTINSXP = 8,
    CHARSXP = 9,
    LGLSXP = 10,
    INTSXP = 13,
    REALSXP = 14,
    CPLXSXP = 15,
    STRSXP = 16,
    DOTSXP = 17,
    ANYSXP = 18,
    VECSXP = 19,
    EXPRSXP = 20,
    BCODESXP = 21,
    EXTPTRSXP = 22,
    WEAKREFSXP = 23,
    RAWSXP = 24,
    S4SXP = 25,
    NEWSXP = 30,
    FREESXP = 31,
}

impl Display for Sexptype {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Sxpinfo {
    pub ty: i32,
    pub scalar: i32,
    pub obj: i32,
    pub alt: i32,
    pub gp: i32,
    pub mark: i32,
    pub debug: i32,
    pub trace: i32,
    pub spare: i32,
    pub gcgen: u32,
    pub gccls: u32,
    pub named: i32,
    pub extra: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SexpPayload {
    Nothing,
    Vecsxp(Vecsxp),
    Primsxp(Primsxp),
    Symsxp(Symsxp),
    Listsxp(Listsxp),
    Envsxp(Envsxp),
    Closxp(Closxp),
    Promsxp(Promsxp),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Vecsxp {
    pub length: i64,
    pub truelength: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Primsxp {
    pub offset: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Symsxp {
    pub pname: Sexp,
    pub value: Sexp,
    pub internal: Sexp,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Listsxp {
    pub carval: Sexp,
    pub cdrval: Sexp,
    pub tagval: Sexp,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Envsxp {
    pub frame: Sexp,
    pub enclos: Sexp,
    pub hashtab: Sexp,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Closxp {
    pub formals: Sexp,
    pub body: Sexp,
    pub env: Sexp,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Promsxp {
    pub value: Sexp,
    pub expr: Sexp,
    pub env: Sexp,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Sexprec {
    pub address: Sexp,
    pub ty: Sexptype,
    pub ty_name: String,
    pub sxpinfo: Sxpinfo,
    pub sxpinfo_bits: u64,
    pub attrib: Sexp,
    pub gengc_next_node: Sexp,
    pub gengc_prev_node: Sexp,
    pub payload: SexpPayload,
}
