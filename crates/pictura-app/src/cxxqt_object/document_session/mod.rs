//! The document session: one live session, one cancel path, one owner of the
//! document's state.
//!
//! `session` is the plain-Rust core (no Qt); `adapter` presents what the
//! session reports. `tests` holds the session's own checks.

mod adapter;
mod session;
#[cfg(test)]
mod tests;

pub(super) use session::{DocumentSession, PendingPreviews, SessionKind};
