//! `pictura --mcp`: hands the process to the stdio bridge. A free function,
//! folded into [`decode_bridge`](crate::decode_bridge) because no Qt object
//! exists yet when it runs.

mod imp;

pub(crate) use imp::*;
