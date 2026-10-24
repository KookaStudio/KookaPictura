//! The document session: one live session, one cancel path, one owner of the
//! document's state.
//!
//! `active` is the slot that holds the document's one live edit session.
//! `session` is the plain-Rust core (no Qt); `adapter` presents what the
//! session reports. `tests` holds the session's own checks.

mod active;
mod adapter;
mod previews;
mod selection;
mod session;
mod stroke;
#[cfg(test)]
mod tests;
mod transform;

pub(super) use active::{PathRestore, SessionKind};
pub(super) use previews::{FilterPreview, FilterPreviewSession, PreviewBaseline};
pub(super) use selection::{SelectionGesture, SelectionMoveState};
pub(super) use session::{DocumentSession, PendingPreviews, QuickSelectionSnapshot};
pub(super) use stroke::{MaskPreview, MaskStroke, PreviewStroke, StrokeSession};
pub(super) use transform::{RepeatTransform, TransformMode, TransformSession, WarpDrag};
