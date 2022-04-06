use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct SexprecHeader {
    pub sxpinfo: Sxpinfo,
    pub sxpinfo_bits: u64,
    pub attrib: u64,
    pub gengc_next_node: u64,
    pub gengc_prev_node: u64,
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
