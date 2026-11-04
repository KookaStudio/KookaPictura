//! Opacity and fill previews and commits: the display-only preview when the
//! bound is the canvas, the exact region/full path otherwise. Both properties
//! share one path; only the setter, preview kind, pending flag, and label
//! differ.

use super::super::display_preview::{commit_display_preview_core, DisplayPreview, PreviewKind};
use super::super::edit::Edit;
use super::super::helpers::list_of_strings;
use super::super::presentation::{Dirty, Presentation, Rebuild};
use super::super::qobject;
use super::super::state::PictureViewRust;
use super::{changed_property_region, property_compositing};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QStringList;
use pictura_core::{Document, PsdRect};

fn set_property(doc: &mut Document, refs: &[&str], value: u8, kind: PreviewKind) -> usize {
    match kind {
        PreviewKind::Opacity => pictura_render::set_opacity_paths(doc, refs, value),
        PreviewKind::Fill => pictura_render::set_fill_paths(doc, refs, value),
        PreviewKind::Adjustment => 0,
    }
}

fn set_pending(rust: &mut PictureViewRust, kind: PreviewKind, value: bool) {
    match kind {
        PreviewKind::Opacity => rust.pending_previews.opacity_changed = value,
        PreviewKind::Fill => rust.pending_previews.fill_changed = value,
        PreviewKind::Adjustment => {}
    }
}

fn pending(rust: &PictureViewRust, kind: PreviewKind) -> bool {
    match kind {
        PreviewKind::Opacity => rust.pending_previews.opacity_changed,
        PreviewKind::Fill => rust.pending_previews.fill_changed,
        PreviewKind::Adjustment => false,
    }
}

fn label(kind: PreviewKind) -> &'static str {
    match kind {
        PreviewKind::Opacity => "Opacity",
        PreviewKind::Fill => "Fill Opacity",
        PreviewKind::Adjustment => "Adjustment",
    }
}

/// The shared layer-property preview. Returns the changed count, the
/// presentation the caller must present, and the display preview the caller
/// must restore *after* presenting (a rebuild clears it).
pub(super) fn preview_layer_property(
    rust: &mut PictureViewRust,
    paths: &[String],
    value: u8,
    kind: PreviewKind,
) -> (i32, Option<Presentation>, Option<DisplayPreview>) {
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    rust.begin_display_preview(paths.to_vec(), kind);
    let Some(doc) = rust.session.doc.as_mut() else {
        return (0, None, None);
    };
    let before: Vec<_> = refs
        .iter()
        .map(|path| property_compositing(doc, path))
        .collect();
    let changed = set_property(doc, &refs, value, kind);
    let region = if changed > 0 {
        changed_property_region(doc, &refs, &before)
    } else {
        Some(None)
    };
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    if changed == 0 {
        return (0, None, None);
    }
    let display = region.is_none() || matches!(region, Some(Some(rect)) if rect == canvas);
    if display {
        if let Some(rect) = rust.present_display_preview() {
            set_pending(rust, kind, true);
            return (
                changed as i32,
                Some(Presentation {
                    regions: vec![rect],
                    rebuild: Rebuild::None,
                    notify: false,
                    dirty: Dirty::Set(true),
                    revision: true,
                    declaration: None,
                }),
                None,
            );
        }
    }
    let saved = rust.display_preview.take();
    let presentation = match region {
        Some(Some(rect)) => {
            let regions = rust.refresh_regions(&[rect]).into_iter().collect();
            Presentation {
                regions,
                rebuild: Rebuild::None,
                notify: false,
                dirty: Dirty::Set(true),
                revision: true,
                declaration: None,
            }
        }
        _ => {
            rust.composite_into_doc();
            Presentation {
                regions: Vec::new(),
                rebuild: Rebuild::Sync,
                notify: true,
                dirty: Dirty::Set(true),
                revision: true,
                declaration: None,
            }
        }
    };
    set_pending(rust, kind, true);
    (changed as i32, Some(presentation), saved)
}

/// The shared layer-property commit. Returns the changed count and the
/// presentation the caller must present.
pub(super) fn commit_layer_property(
    rust: &mut PictureViewRust,
    paths: &[String],
    value: u8,
    kind: PreviewKind,
) -> (i32, Option<Presentation>) {
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    if rust.display_preview_active() {
        let changed = {
            let Some(doc) = rust.session.doc.as_mut() else {
                return (0, None);
            };
            set_property(doc, &refs, value, kind)
        };
        let recorded = commit_display_preview_core(rust, label(kind));
        set_pending(rust, kind, false);
        return match recorded {
            Some((true, presentation)) => (paths.len() as i32, Some(presentation)),
            Some((false, presentation)) => (0, Some(presentation)),
            None => (changed as i32, None),
        };
    }
    let pending = pending(rust, kind);
    let (changed, edit) = {
        let Some(doc) = rust.session.doc.as_mut() else {
            return (0, None);
        };
        let before: Vec<_> = refs
            .iter()
            .map(|path| property_compositing(doc, path))
            .collect();
        let changed = set_property(doc, &refs, value, kind);
        let edit = if changed == 0 {
            if pending {
                Edit::Record
            } else {
                Edit::None
            }
        } else {
            match changed_property_region(doc, &refs, &before) {
                Some(Some(rect)) => Edit::regions(vec![rect], true),
                _ => Edit::Full,
            }
        };
        (changed, edit)
    };
    let presentation = rust.apply_edit(label(kind), edit);
    set_pending(rust, kind, false);
    (changed as i32, Some(presentation))
}

impl qobject::PictureView {
    pub fn preview_layers_opacity(
        mut self: Pin<&mut Self>,
        paths: &QStringList,
        value: i32,
    ) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let (changed, presentation, saved) = {
            let mut rust = self.as_mut().rust_mut();
            preview_layer_property(&mut rust, &owned, value, PreviewKind::Opacity)
        };
        if let Some(presentation) = presentation {
            self.as_mut().present(presentation);
        }
        if let Some(saved) = saved {
            self.as_mut().rust_mut().display_preview = Some(saved);
        }
        changed
    }

    pub fn commit_layers_opacity(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let (changed, presentation) = {
            let mut rust = self.as_mut().rust_mut();
            commit_layer_property(&mut rust, &owned, value, PreviewKind::Opacity)
        };
        if let Some(presentation) = presentation {
            self.as_mut().present(presentation);
        }
        changed
    }

    pub fn preview_layers_fill(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let (changed, presentation, saved) = {
            let mut rust = self.as_mut().rust_mut();
            preview_layer_property(&mut rust, &owned, value, PreviewKind::Fill)
        };
        if let Some(presentation) = presentation {
            self.as_mut().present(presentation);
        }
        if let Some(saved) = saved {
            self.as_mut().rust_mut().display_preview = Some(saved);
        }
        changed
    }

    pub fn commit_layers_fill(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let (changed, presentation) = {
            let mut rust = self.as_mut().rust_mut();
            commit_layer_property(&mut rust, &owned, value, PreviewKind::Fill)
        };
        if let Some(presentation) = presentation {
            self.as_mut().present(presentation);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::document_session::DocumentSession;
    use crate::cxxqt_object::presentation::{present_core, PresentStep};
    use crate::cxxqt_object::tests::pixel_layer;
    use pictura_core::{BitDepth, ColorMode, Document, LayerStack, PsdRect};

    fn view() -> PictureViewRust {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(vec![pixel_layer("base", 4, 4, (255, 0, 0))]);
        doc.composite = pictura_render::composite_rgba(&doc);
        let mut view = PictureViewRust {
            session: DocumentSession::with_document(doc),
            ..Default::default()
        };
        view.layer_target = crate::layer_target::LayerTarget::only("0");
        view.presented_level = 0;
        view.presented_rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        };
        view.reset_pyramid();
        view
    }

    #[test]
    fn display_preview_uses_presenter() {
        let mut view = view();
        let (changed, presentation, saved) =
            preview_layer_property(&mut view, &["0".to_string()], 128, PreviewKind::Opacity);
        assert!(changed > 0);
        let presentation = presentation.expect("a presentation");
        let steps = present_core(&mut view, &presentation);
        if let Some(saved) = saved {
            view.display_preview = Some(saved);
        }
        assert!(
            steps
                .iter()
                .any(|step| matches!(step, PresentStep::Blit(_))),
            "the presenter emits the region: {steps:?}"
        );
        assert!(
            !steps.contains(&PresentStep::Notify),
            "a preview does not notify: {steps:?}"
        );
    }

    #[test]
    fn opacity_and_fill_share_path() {
        let mut opacity = view();
        let (opacity_changed, opacity_presentation, opacity_saved) =
            preview_layer_property(&mut opacity, &["0".to_string()], 128, PreviewKind::Opacity);
        let mut fill = view();
        let (fill_changed, fill_presentation, fill_saved) =
            preview_layer_property(&mut fill, &["0".to_string()], 128, PreviewKind::Fill);
        if let Some(saved) = opacity_saved {
            opacity.display_preview = Some(saved);
        }
        if let Some(saved) = fill_saved {
            fill.display_preview = Some(saved);
        }
        assert!(opacity_changed > 0 && fill_changed > 0);
        assert!(opacity_presentation.is_some() && fill_presentation.is_some());
    }
}
