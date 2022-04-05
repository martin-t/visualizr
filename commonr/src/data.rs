use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct SexprecHeader {
    pub sxpinfo: u64,
    pub attrib: u64,
    pub gengc_next_node: u64,
    pub gengc_prev_node: u64,
}
