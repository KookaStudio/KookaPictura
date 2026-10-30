//! Display-only live previews.
//!
//! A preview patches only the display levels (level 0's visible rectangle or a
//! stored pyramid level) and leaves `doc.composite` alone. The commit that ends
//! the gesture composites the exact frame and records one history state.

use super::edit::Edit;
use super::helpers_composite::{display_rgba_bytes, patch_buffer_region};
use super::qobject::PictureView;
use super::state::PictureViewRust;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::{Document, Layer, PsdRect};

/// The property a display preview is changing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PreviewKind {
    Adjustment,
    Opacity,
    Fill,
}

/// A live display preview: the targets, the pre-gesture layers, and the
/// document generation it belongs to.
pub(super) struct DisplayPreview {
    pub(super) paths: Vec<String>,
    pub(super) kind: PreviewKind,
    pub(super) pre_layers: Vec<(String, Layer)>,
    pub(super) presented: bool,
    /// The view-pyramid level the preview was last presented at.
    pub(super) level: Option<u32>,
    pub(super) generation: u64,
}

impl DisplayPreview {
    /// Whether the committed values equal the values before the gesture.
    fn unchanged(&self, doc: Option<&Document>) -> bool {
        let Some(doc) = doc else {
            return false;
        };
        self.pre_layers.iter().all(|(path, before)| {
            let Some(now) = pictura_render::resolve_path(doc, path) else {
                return false;
            };
            match self.kind {
                PreviewKind::Adjustment => before.adjustment == now.adjustment,
                PreviewKind::Opacity => before.opacity == now.opacity,
                PreviewKind::Fill => before.fill == now.fill,
            }
        })
    }

    /// The exact edit that finishes the gesture: the union of the targets'
    /// damage bounds, or the full composite when any is unbounded.
    fn exact_edit(&self, doc: Option<&Document>) -> Edit {
        let Some(doc) = doc else {
            return Edit::Full;
        };
        // Above level 0 the preview patched a reduced level; only a full
        // composite makes every stored level exact again.
        if self.level.is_some_and(|level| level > 0) {
            return Edit::Full;
        }
        let mut rects = Vec::with_capacity(self.paths.len());
        for path in &self.paths {
            match pictura_render::damage::influence(doc, path) {
                Some(rect) => rects.push(rect),
                None => return Edit::Full,
            }
        }
        Edit::regions(rects, true)
    }
}

impl PictureViewRust {
    /// Start (or continue) a display preview. The first call of a gesture
    /// captures the targets' pre-gesture layers for the commit's no-op check.
    pub(super) fn begin_display_preview(&mut self, paths: Vec<String>, kind: PreviewKind) {
        let generation = self.session.document_generation;
        let matches = self.display_preview.as_ref().is_some_and(|preview| {
            preview.kind == kind && preview.paths == paths && preview.generation == generation
        });
        if matches {
            return;
        }
        let pre_layers = self.session.doc.as_ref().map_or_else(Vec::new, |doc| {
            paths
                .iter()
                .filter_map(|path| {
                    pictura_render::resolve_path(doc, path)
                        .cloned()
                        .map(|layer| (path.clone(), layer))
                })
                .collect()
        });
        self.display_preview = Some(DisplayPreview {
            paths,
            kind,
            pre_layers,
            presented: false,
            level: None,
            generation,
        });
    }

    /// Composite and present the current preview at the level and rectangle the
    /// canvas last reported. `None` when the exact path must run instead.
    pub(super) fn present_display_preview(&mut self) -> Option<PsdRect> {
        if self.has_active_session() || self.mask_solo.is_some() {
            return None;
        }
        let preview = self.display_preview.as_ref()?;
        let level = self.presented_level;
        if level < 0 {
            return None;
        }
        let doc = self.session.doc.as_ref()?;
        // The canvas reports a rect rounded out to the level grid, which can
        // overshoot the document by up to 2^level - 1 pixels. Clamp it so the
        // reduced frame and the level rect have the same size.
        let rect = clamp_rect(self.presented_rect, doc.width, doc.height);
        if rect.width() <= 0 || rect.height() <= 0 {
            return None;
        }
        let key = self.preview_key();
        let targets: Vec<&str> = preview.paths.iter().map(String::as_str).collect();
        let frame = self
            .render_session
            .preview(doc, &targets, level as u32, rect, key)?;
        let srgb = pictura_codec::buffer_to_srgb(doc, &frame).into_owned();
        if level == 0 {
            let level0 = self.level0.as_mut()?;
            patch_buffer_region(level0, &srgb, rect.left, rect.top);
        } else {
            let rgba = display_rgba_bytes(&srgb);
            let scale = 1i32 << level;
            let level_rect = PsdRect {
                top: rect.top / scale,
                left: rect.left / scale,
                bottom: rect.bottom / scale,
                right: rect.right / scale,
            };
            self.pyramid.patch_level(level as usize, &rgba, level_rect);
        }
        self.canvas_revision = self.canvas_revision.wrapping_add(1);
        self.damage.mark(rect);
        if let Some(preview) = self.display_preview.as_mut() {
            preview.presented = true;
            preview.level = Some(level as u32);
        }
        Some(rect)
    }

    pub(super) fn clear_display_preview(&mut self) {
        self.display_preview = None;
    }

    /// Abandon the live preview: put the pre-gesture layers back, then restore
    /// the exact display from the unchanged document composite.
    pub(super) fn cancel_display_preview(&mut self) {
        let Some(preview) = self.display_preview.take() else {
            return;
        };
        if let Some(doc) = self.session.doc.as_mut() {
            for (path, before) in &preview.pre_layers {
                if let Some(layer) = pictura_render::resolve_path_mut(doc, path) {
                    *layer = before.clone();
                }
            }
        }
        self.reset_pyramid();
    }

    pub(super) fn display_preview_active(&self) -> bool {
        self.display_preview.is_some()
    }

    /// Whether the live preview actually patched a display level. A fallback
    /// preview keeps the pre-gesture state but leaves the composite exact.
    pub(super) fn display_preview_presented(&self) -> bool {
        self.display_preview
            .as_ref()
            .is_some_and(|preview| preview.presented)
    }

    fn preview_key(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.session.document_generation.hash(&mut hasher);
        self.non_preview_revision.hash(&mut hasher);
        hasher.finish()
    }
}

fn clamp_rect(rect: PsdRect, width: u32, height: u32) -> PsdRect {
    PsdRect {
        top: rect.top.max(0),
        left: rect.left.max(0),
        bottom: rect.bottom.min(height.min(i32::MAX as u32) as i32),
        right: rect.right.min(width.min(i32::MAX as u32) as i32),
    }
}

/// Finish the live display preview: composite the exact frame and record one
/// history state, or restore the display and record nothing when the value
/// returned to its pre-gesture value.
///
/// `None` when there is no display-preview state; `Some(false)` when the value
/// is unchanged and nothing was recorded; `Some(true)` when a state was
/// recorded.
pub(super) fn commit_display_preview(mut view: Pin<&mut PictureView>, label: &str) -> Option<bool> {
    let (edit, unchanged, presented) = {
        let mut rust = view.as_mut().rust_mut();
        let preview = rust.display_preview.take()?;
        let doc = rust.session.doc.as_ref();
        let unchanged = preview.unchanged(doc);
        let edit = preview.exact_edit(doc);
        (edit, unchanged, preview.presented)
    };
    if unchanged {
        view.as_mut().rust_mut().reset_pyramid();
        view.as_mut().changed();
        return Some(false);
    }
    if presented {
        view.as_mut().edit(label, |_| edit);
    } else {
        // The fallback already composited the exact frame; capture it without
        // compositing again.
        view.as_mut().edit(label, |_| Edit::Record);
    }
    Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::document_session::DocumentSession;
    use crate::cxxqt_object::edit::Edit;
    use pictura_core::Document;

    fn document() -> Document {
        Document::from_rgba("d", 8, 8, &[255u8; 256])
    }

    fn view_of(doc: Document) -> PictureViewRust {
        let mut view = PictureViewRust {
            session: DocumentSession::with_document(doc),
            ..Default::default()
        };
        view.presented_level = 0;
        view.presented_rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 8,
            right: 8,
        };
        view.reset_pyramid();
        view
    }

    #[test]
    fn preview_patches_only_the_presented_level() {
        let mut view = view_of(document());
        view.presented_rect = PsdRect {
            top: 2,
            left: 2,
            bottom: 6,
            right: 6,
        };
        let before = view.level0.as_ref().expect("level 0").clone();
        let composite = view.session.doc.as_ref().expect("doc").composite.clone();
        view.begin_display_preview(vec!["0".to_string()], PreviewKind::Opacity);
        view.session.doc.as_mut().expect("doc").layers[0].opacity = 128;
        let rect = view.present_display_preview().expect("preview");
        assert_eq!(rect, view.presented_rect);
        let after = view.level0.as_ref().expect("level 0");
        assert_eq!(after.data[0], before.data[0], "outside the rect");
        assert_eq!(after.data[7], before.data[7], "outside the rect");
        let inside = 3 * 64 + 2 * 8 + 2;
        assert_ne!(after.data[inside], before.data[inside], "inside alpha");
        assert_eq!(view.session.doc.as_ref().expect("doc").composite, composite);
    }

    #[test]
    fn preview_leaves_the_composite_unchanged() {
        let mut view = view_of(document());
        let composite = view.session.doc.as_ref().expect("doc").composite.clone();
        view.begin_display_preview(vec!["0".to_string()], PreviewKind::Opacity);
        view.session.doc.as_mut().expect("doc").layers[0].opacity = 128;
        assert!(view.present_display_preview().is_some());
        assert_eq!(view.session.doc.as_ref().expect("doc").composite, composite);
    }

    #[test]
    fn commit_produces_the_exact_frame() {
        let mut view = view_of(document());
        view.begin_display_preview(vec!["0".to_string()], PreviewKind::Opacity);
        view.session.doc.as_mut().expect("doc").layers[0].opacity = 128;
        assert!(view.present_display_preview().is_some());
        let preview = view.display_preview.take().expect("preview");
        let edit = preview.exact_edit(view.session.doc.as_ref());
        view.apply_edit("Opacity", edit);
        let doc = view.session.doc.as_ref().expect("doc");
        assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
        assert_eq!(view.session.history.count(), 1);
    }

    #[test]
    fn display_preview_commit_follows_the_bound() {
        let mut view = view_of(document());
        let bounded = {
            let mut doc = view.session.doc.take().expect("doc");
            let path =
                pictura_render::add_raster_layer_from_rgba(&mut doc, "small", 2, 2, &[0u8; 16]);
            view.session.doc = Some(doc);
            path
        };
        view.begin_display_preview(vec![bounded], PreviewKind::Opacity);
        let preview = view.display_preview.as_ref().expect("preview");
        assert!(matches!(
            preview.exact_edit(view.session.doc.as_ref()),
            Edit::Regions { .. }
        ));

        let path = {
            let mut doc = view.session.doc.take().expect("doc");
            let index = pictura_render::add_layer(&mut doc, -1, "adjustment");
            let path = index.to_string();
            if let Some(layer) = pictura_render::resolve_path_mut(&mut doc, &path) {
                layer.adjustment = Some(pictura_render::encode_invert());
            }
            view.session.doc = Some(doc);
            path
        };
        view.begin_display_preview(vec![path], PreviewKind::Adjustment);
        let preview = view.display_preview.as_ref().expect("preview");
        assert!(matches!(
            preview.exact_edit(view.session.doc.as_ref()),
            Edit::Full
        ));
    }

    #[test]
    fn unchanged_preview_records_nothing() {
        let mut view = view_of(document());
        view.begin_display_preview(vec!["0".to_string()], PreviewKind::Opacity);
        let preview = view.display_preview.as_ref().expect("preview");
        assert!(preview.unchanged(view.session.doc.as_ref()));
    }

    #[test]
    fn noop_round_trip_records_nothing() {
        let mut view = view_of(document());
        view.begin_display_preview(vec!["0".to_string()], PreviewKind::Opacity);
        view.session.doc.as_mut().expect("doc").layers[0].opacity = 128;
        view.session.doc.as_mut().expect("doc").layers[0].opacity = 255;
        let preview = view.display_preview.as_ref().expect("preview");
        assert!(preview.unchanged(view.session.doc.as_ref()));
    }

    #[test]
    fn no_presented_level_falls_back_to_exact() {
        let mut view = view_of(document());
        view.presented_level = -1;
        view.begin_display_preview(vec!["0".to_string()], PreviewKind::Opacity);
        assert!(view.present_display_preview().is_none());
    }

    #[test]
    fn no_stored_level_falls_back_to_exact() {
        let mut view = view_of(document());
        view.presented_level = 99;
        view.begin_display_preview(vec!["0".to_string()], PreviewKind::Opacity);
        assert!(view.present_display_preview().is_none());
    }

    #[test]
    fn preview_patches_the_presented_stored_level() {
        let doc = Document::from_rgba("d", 1024, 1024, &vec![255u8; 1024 * 1024 * 4]);
        let mut view = PictureViewRust {
            session: DocumentSession::with_document(doc),
            ..Default::default()
        };
        view.presented_level = 2;
        view.presented_rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 1024,
            right: 1024,
        };
        view.reset_pyramid();
        let whole = PsdRect {
            top: 0,
            left: 0,
            bottom: 256,
            right: 256,
        };
        let before = {
            let planes =
                super::super::helpers_composite::planes_of(view.level0.as_ref().expect("level 0"))
                    .expect("planes");
            view.pyramid.crop(planes, 2, whole)
        };
        view.begin_display_preview(vec!["0".to_string()], PreviewKind::Opacity);
        view.session.doc.as_mut().expect("doc").layers[0].opacity = 128;
        let rect = view.present_display_preview().expect("preview");
        assert_eq!(rect, view.presented_rect);
        let after = {
            let planes =
                super::super::helpers_composite::planes_of(view.level0.as_ref().expect("level 0"))
                    .expect("planes");
            view.pyramid.crop(planes, 2, whole)
        };
        assert_ne!(before.data(), after.data(), "stored level 2 was patched");
    }

    #[test]
    fn solo_mask_and_live_stroke_fall_back() {
        let mut view = view_of(document());
        view.mask_solo = Some("0".to_string());
        view.begin_display_preview(vec!["0".to_string()], PreviewKind::Opacity);
        assert!(view.present_display_preview().is_none());
    }

    #[test]
    fn cancel_restores_the_pre_gesture_display() {
        let doc = Document::from_rgba("d", 4, 4, &[255u8; 64]);
        let mut view = PictureViewRust {
            session: DocumentSession::with_document(doc),
            ..Default::default()
        };
        view.presented_level = 0;
        view.presented_rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        };
        view.reset_pyramid();
        let before = view.level0.as_ref().expect("level 0").clone();
        view.begin_display_preview(vec!["0".to_string()], PreviewKind::Opacity);
        view.session.doc.as_mut().expect("doc").layers[0].opacity = 128;
        assert!(view.present_display_preview().is_some());
        assert_ne!(view.level0.as_ref().expect("level 0"), &before);
        view.cancel_display_preview();
        assert_eq!(view.level0.as_ref().expect("level 0"), &before);
        assert!(!view.display_preview_active());
    }
}
