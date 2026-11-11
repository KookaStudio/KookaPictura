//! One change report and one presenter.
//!
//! Every producer reports the pixels it already folded into the display, the
//! rebuild it needs, and the bookkeeping to apply. This module owns the order:
//! bookkeeping, then regions, then at most one rebuild, then at most one
//! notification. The pure [`plan`] is what the Rust tests assert; `present`
//! executes it against the QObject.

use super::edit::Edit;
use super::qobject;
use super::state::PictureViewRust;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::PsdRect;

/// The display work a presentation needs after its regions are emitted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Rebuild {
    /// The display is already current.
    #[default]
    None,
    /// Rebuild the display from the exact document composite.
    Sync,
    /// Rebuild a large display on a worker.
    Deferred,
}

/// The dirty write a presentation performs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Dirty {
    /// Leave the dirty flag alone.
    #[default]
    Unchanged,
    /// Set the dirty flag to this value.
    Set(bool),
}

/// What a producer changed, in the one shape every presenter consumes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Presentation {
    /// Rectangles whose pixels are already folded into level 0 and the pyramid.
    pub(super) regions: Vec<PsdRect>,
    pub(super) rebuild: Rebuild,
    pub(super) notify: bool,
    pub(super) dirty: Dirty,
    pub(super) revision: bool,
    /// The normalized edit declaration, for the transaction's callers only.
    pub(super) declaration: Option<Edit>,
}

impl Presentation {
    pub(super) fn none() -> Self {
        Self::default()
    }

    pub(super) fn notify() -> Self {
        Self {
            notify: true,
            ..Self::default()
        }
    }

    pub(super) fn rebuild(kind: Rebuild) -> Self {
        Self {
            rebuild: kind,
            notify: true,
            ..Self::default()
        }
    }

    pub(super) fn regions(regions: Vec<PsdRect>, notify: bool) -> Self {
        Self {
            regions,
            notify,
            ..Self::default()
        }
    }

    /// Whether the shell must redraw or refresh after this presentation.
    ///
    /// An edit `Regions { rects: [], hint: Some(_) }` reports no display change
    /// even though the old `Effect::Blit { notify: false }` did: the producer
    /// folded no pixels and emits no signal. No caller applies this to an edit
    /// presentation; it is used for session reports, where an empty region list
    /// is a genuine no-op.
    pub(super) fn has_display_change(&self) -> bool {
        self.rebuild != Rebuild::None || !self.regions.is_empty() || self.notify
    }

    /// Whether this presentation asks the shell for nothing.
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        !self.has_display_change() && matches!(self.dirty, Dirty::Unchanged) && !self.revision
    }
}

/// One executed presentation step. The Qt layer turns `Blit` and `Notify` into
/// signals; `Bookkeep` and `Rebuild` are already applied by `present_core`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PresentStep {
    Bookkeep,
    Blit(PsdRect),
    Rebuild,
    Notify,
}

/// The fixed order a presentation executes in.
pub(super) fn plan(presentation: &Presentation) -> Vec<PresentStep> {
    let mut steps = Vec::new();
    if !matches!(presentation.dirty, Dirty::Unchanged) || presentation.revision {
        steps.push(PresentStep::Bookkeep);
    }
    for &rect in &presentation.regions {
        steps.push(PresentStep::Blit(rect));
    }
    if presentation.rebuild != Rebuild::None {
        steps.push(PresentStep::Rebuild);
    }
    if presentation.notify || presentation.rebuild != Rebuild::None {
        steps.push(PresentStep::Notify);
    }
    steps
}

/// Apply the bookkeeping and rebuild halves of `presentation`, returning the
/// order the Qt layer must replay for the signal half.
pub(super) fn present_core(
    rust: &mut PictureViewRust,
    presentation: &Presentation,
) -> Vec<PresentStep> {
    let steps = plan(presentation);
    for step in &steps {
        match step {
            PresentStep::Bookkeep => {
                if let Dirty::Set(dirty) = presentation.dirty {
                    rust.session.dirty = dirty;
                }
                if presentation.revision {
                    rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
                }
            }
            PresentStep::Blit(_) => {}
            PresentStep::Rebuild => match presentation.rebuild {
                Rebuild::None => {}
                // The tiles rebuild on request either way; a large document
                // builds the visible ones on a worker (#1216).
                Rebuild::Sync | Rebuild::Deferred => rust.reset_pyramid(),
            },
            PresentStep::Notify => {}
        }
    }
    steps
}

impl qobject::PictureView {
    /// Present a producer's report: bookkeeping, regions, rebuild, notify.
    pub(super) fn present(mut self: Pin<&mut Self>, presentation: Presentation) {
        let steps = {
            let mut rust = self.as_mut().rust_mut();
            present_core(&mut rust, &presentation)
        };
        for step in steps {
            match step {
                PresentStep::Bookkeep => {}
                PresentStep::Notify => self.as_mut().changed(),
                PresentStep::Blit(rect) => {
                    if rect.width() > 0 && rect.height() > 0 {
                        self.as_mut().region_damaged(
                            rect.left,
                            rect.top,
                            rect.width(),
                            rect.height(),
                        );
                    }
                }
                PresentStep::Rebuild => {}
            }
        }
    }
}

#[cfg(test)]
#[path = "presentation_tests.rs"]
mod presentation_tests;
