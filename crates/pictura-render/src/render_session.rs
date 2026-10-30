//! The interactive render session: a live preview from the visible region at
//! the presented level, reusing the composite below the edited layer.
//!
//! The session owns the backdrop slot, the prefix digest, and the reduced
//! preview document. It never touches the document's maintained composite or
//! the display; the app patches the frame it returns.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use pictura_core::{BitDepth, ColorMode, Document, Layer, PixelBuffer, PsdRect};

use crate::composite::Canvas;
use crate::composite_knockout::composite_layers_range;
use crate::composite_route::{content_route, ContentRoute};
use crate::filter_proxy::FilterProxy;
use crate::layer_source::key::content_key;
use crate::view_pyramid::SMALLEST_SIDE;

/// The default preview backdrop budget: the backdrop plus its scratch canvas.
pub const DEFAULT_PREVIEW_BUDGET: usize = 64 * 1024 * 1024;

/// A preview session. Create one per view and reuse it across gestures.
pub struct RenderSession {
    backdrop: Option<Backdrop>,
    preview_doc: Option<PreviewDoc>,
    budget: usize,
    backdrop_hits: u64,
    backdrop_builds: u64,
    preview_doc_builds: u64,
}

impl Default for RenderSession {
    fn default() -> Self {
        Self::with_budget(DEFAULT_PREVIEW_BUDGET)
    }
}

struct Backdrop {
    key: u64,
    level: u32,
    region: PsdRect,
    canvas: Canvas,
}

struct PreviewDoc {
    key: u64,
    doc: Document,
}

impl RenderSession {
    /// A session that admits a backdrop plus scratch of at most `budget` bytes.
    pub fn with_budget(budget: usize) -> Self {
        Self {
            backdrop: None,
            preview_doc: None,
            budget,
            backdrop_hits: 0,
            backdrop_builds: 0,
            preview_doc_builds: 0,
        }
    }

    /// The preview frame for `targets` at `level` over `rect`, or `None` when
    /// the exact path must run.
    pub fn preview(
        &mut self,
        doc: &Document,
        targets: &[&str],
        level: u32,
        rect: PsdRect,
        key: u64,
    ) -> Option<PixelBuffer> {
        if rect.width() <= 0 || rect.height() <= 0 || targets.is_empty() {
            return None;
        }
        if !level_exists(doc, level) {
            return None;
        }
        let k = targets
            .iter()
            .map(|target| resume_index(doc, target))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .min()?;
        let digest = prefix_digest(doc, k, level, rect);
        if level == 0 {
            let mut scratch = backdrop_for(
                &mut self.backdrop,
                self.budget,
                &mut self.backdrop_hits,
                &mut self.backdrop_builds,
                BackdropRequest {
                    doc,
                    k,
                    level,
                    rect,
                    digest,
                },
            )?;
            scratch.clip = rect;
            scratch.effect_region = Some(rect);
            composite_layers_range(&mut scratch, doc, k, doc.layers.len());
            return Some(scratch.into_pixel_buffer());
        }
        if !reducible(doc) {
            return None;
        }
        self.ensure_preview_document(doc, level, rect, key, digest)?;
        {
            let reduced = &mut self.preview_doc.as_mut()?.doc;
            sync_targets(reduced, doc, targets);
        }
        let reduced = &self.preview_doc.as_ref()?.doc;
        let reduced_rect = PsdRect {
            top: 0,
            left: 0,
            bottom: reduced.height as i32,
            right: reduced.width as i32,
        };
        let mut scratch = backdrop_for(
            &mut self.backdrop,
            self.budget,
            &mut self.backdrop_hits,
            &mut self.backdrop_builds,
            BackdropRequest {
                doc: reduced,
                k,
                level,
                rect: reduced_rect,
                digest,
            },
        )?;
        scratch.clip = reduced_rect;
        scratch.effect_region = Some(reduced_rect);
        composite_layers_range(&mut scratch, reduced, k, reduced.layers.len());
        Some(scratch.into_pixel_buffer())
    }

    /// How many preview frames reused the cached backdrop.
    pub fn backdrop_hits(&self) -> u64 {
        self.backdrop_hits
    }

    /// How many times the backdrop was composited.
    pub fn backdrop_builds(&self) -> u64 {
        self.backdrop_builds
    }

    /// How many times the reduced document was built.
    pub fn preview_doc_builds(&self) -> u64 {
        self.preview_doc_builds
    }

    fn ensure_preview_document(
        &mut self,
        doc: &Document,
        level: u32,
        rect: PsdRect,
        key: u64,
        digest: u64,
    ) -> Option<()> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        digest.hash(&mut hasher);
        level.hash(&mut hasher);
        rect.left.hash(&mut hasher);
        rect.top.hash(&mut hasher);
        rect.right.hash(&mut hasher);
        rect.bottom.hash(&mut hasher);
        let doc_key = hasher.finish();
        if self
            .preview_doc
            .as_ref()
            .is_some_and(|preview| preview.key == doc_key)
        {
            return Some(());
        }
        let scale = 1.0 / f64::from(1u32 << level);
        let reduced = FilterProxy::new(doc, None, rect, scale)?.into_document();
        self.preview_doc = Some(PreviewDoc {
            key: doc_key,
            doc: reduced,
        });
        self.preview_doc_builds += 1;
        Some(())
    }
}

struct BackdropRequest<'a> {
    doc: &'a Document,
    k: usize,
    level: u32,
    rect: PsdRect,
    digest: u64,
}

fn backdrop_for(
    slot: &mut Option<Backdrop>,
    budget: usize,
    hits: &mut u64,
    builds: &mut u64,
    request: BackdropRequest<'_>,
) -> Option<Canvas> {
    let BackdropRequest {
        doc,
        k,
        level,
        rect,
        digest,
    } = request;
    if let Some(backdrop) = slot.as_ref() {
        if backdrop.key == digest && backdrop.level == level && backdrop.region == rect {
            *hits += 1;
            return Some(backdrop.canvas.clone());
        }
    }
    let pixels = rect.width().max(0) as usize * rect.height().max(0) as usize;
    // The backdrop and its scratch copy are 16 bytes/pixel each; the returned
    // frame is another 4 bytes/pixel while the scratch is still alive.
    if pixels.saturating_mul(16 * 2 + 4) > budget {
        return None;
    }
    let mut canvas = Canvas::new_region(
        rect.left,
        rect.top,
        rect.width() as usize,
        rect.height() as usize,
    );
    canvas.clip = rect;
    canvas.effect_region = Some(rect);
    composite_layers_range(&mut canvas, doc, 0, k);
    *builds += 1;
    *slot = Some(Backdrop {
        key: digest,
        level,
        region: rect,
        canvas: canvas.clone(),
    });
    Some(canvas)
}

/// The root index a resume may start at, or `None` when the target is nested,
/// clipped, or the document cannot be resumed.
fn resume_index(doc: &Document, target: &str) -> Option<usize> {
    if target.contains('/') {
        return None;
    }
    let k: usize = target.parse().ok()?;
    let layer = doc.layers.get(k)?;
    if layer.clipping {
        return None;
    }
    if doc.mode != ColorMode::Rgb || doc.depth != BitDepth::Eight || doc.source_depth.is_some() {
        return None;
    }
    Some(k)
}

/// Whether every layer can be reduced faithfully. Live type, proxy-less smart
/// objects, and undecoded vector fills lose their rect in the proxy, so a stack
/// containing one falls back to the exact path above level 0.
fn reducible(doc: &Document) -> bool {
    fn walk(layer: &Layer) -> bool {
        match content_route(layer) {
            ContentRoute::TypeSource
            | ContentRoute::SmartSource
            | ContentRoute::UndecodedVectorFill => false,
            _ => layer.children.iter().all(walk),
        }
    }
    doc.layers.iter().all(walk)
}

/// Whether `level` exists in the view pyramid for `doc`.
fn level_exists(doc: &Document, level: u32) -> bool {
    if level == 0 {
        return true;
    }
    let mut long = doc.width.max(doc.height);
    for _ in 0..level {
        long = long.div_ceil(2);
        if long < SMALLEST_SIDE {
            return false;
        }
    }
    true
}

fn sync_targets(reduced: &mut Document, original: &Document, targets: &[&str]) {
    for target in targets {
        let Some(source) = crate::resolve_path(original, target) else {
            continue;
        };
        let Some(destination) = crate::resolve_path_mut(reduced, target) else {
            continue;
        };
        destination.adjustment = source.adjustment.clone();
        destination.opacity = source.opacity;
        destination.fill = source.fill;
        destination.blend = source.blend;
        destination.visible = source.visible;
        destination.clipping = source.clipping;
        destination.knockout = source.knockout;
    }
}

fn prefix_digest(doc: &Document, k: usize, level: u32, rect: PsdRect) -> u64 {
    let mut hasher = DefaultHasher::new();
    doc.width.hash(&mut hasher);
    doc.height.hash(&mut hasher);
    doc.mode.hash(&mut hasher);
    // The pattern library lives in the layer-section extra bytes and is not
    // part of any layer's content key; a pattern edit must invalidate.
    doc.layer_section_extra.ident().hash(&mut hasher);
    level.hash(&mut hasher);
    rect.left.hash(&mut hasher);
    rect.top.hash(&mut hasher);
    rect.right.hash(&mut hasher);
    rect.bottom.hash(&mut hasher);
    for layer in doc.layers.iter().take(k) {
        layer_digest(layer, doc, &mut hasher);
    }
    hasher.finish()
}

fn layer_digest(layer: &Layer, doc: &Document, hasher: &mut DefaultHasher) {
    content_key(layer, doc).hash(hasher);
    layer.blend.name().hash(hasher);
    layer.opacity.hash(hasher);
    layer.fill.hash(hasher);
    layer.visible.hash(hasher);
    layer.clipping.hash(hasher);
    (layer.knockout as u8).hash(hasher);
    layer.is_group.hash(hasher);
    layer.blending_ranges.ident().hash(hasher);
    layer.blend_if.is_some().hash(hasher);
    layer.blend_clipping.hash(hasher);
    layer.blend_interior.hash(hasher);
    layer.background.hash(hasher);
    layer.blend_key.hash(hasher);
    for child in &layer.children {
        layer_digest(child, doc, hasher);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, BlendMode, Channel, Knockout, LayerStack};

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom,
            right,
        }
    }

    fn whole(doc: &Document) -> PsdRect {
        rect(0, 0, doc.width as i32, doc.height as i32)
    }

    fn layer(name: &str, at: PsdRect, rgba: [u8; 4]) -> Layer {
        let n = (at.width().max(0) * at.height().max(0)) as usize;
        Layer {
            name: name.into(),
            rect: at,
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            visible: true,
            channels: [0i16, 1, 2, -1]
                .into_iter()
                .zip(rgba)
                .map(|(id, value)| Channel {
                    id,
                    data: vec![value; n].into(),
                })
                .collect(),
            ..Default::default()
        }
    }

    fn document(width: u32, height: u32, layers: Vec<Layer>) -> Document {
        let mut doc = Document::new(width, height, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(layers);
        doc
    }

    fn two_layers() -> Document {
        document(
            8,
            8,
            vec![
                layer("bottom", rect(0, 0, 8, 8), [30, 60, 90, 255]),
                layer("top", rect(2, 2, 6, 6), [200, 100, 50, 255]),
            ],
        )
    }

    /// A base that covers only the left half, so clipping visibly changes the
    /// composite.
    fn clipped_document() -> Document {
        document(
            8,
            8,
            vec![
                layer("bottom", rect(0, 0, 4, 8), [30, 60, 90, 255]),
                layer("top", rect(2, 2, 6, 6), [200, 100, 50, 255]),
            ],
        )
    }

    #[test]
    fn prefix_digest_changes_on_a_pixel_edit() {
        let mut doc = two_layers();
        let before = prefix_digest(&doc, 1, 0, whole(&doc));
        doc.layers[0].channels[0].data = vec![1u8; 64].into();
        assert_ne!(before, prefix_digest(&doc, 1, 0, whole(&doc)));
    }

    #[test]
    fn prefix_digest_changes_on_a_property_edit() {
        let mut doc = two_layers();
        let before = prefix_digest(&doc, 1, 0, whole(&doc));
        doc.layers[0].opacity = 128;
        assert_ne!(before, prefix_digest(&doc, 1, 0, whole(&doc)));
    }

    #[test]
    fn prefix_digest_changes_on_a_reorder() {
        let mut doc = two_layers();
        let before = prefix_digest(&doc, 1, 0, whole(&doc));
        let mut layers: Vec<Layer> = doc.layers.iter().cloned().collect();
        layers.swap(0, 1);
        doc.layers = LayerStack::from_layers(layers);
        assert_ne!(before, prefix_digest(&doc, 1, 0, whole(&doc)));
    }

    #[test]
    fn prefix_digest_changes_on_a_pattern_edit() {
        let mut doc = two_layers();
        let before = prefix_digest(&doc, 1, 0, whole(&doc));
        doc.layer_section_extra = vec![1u8, 2, 3].into();
        assert_ne!(before, prefix_digest(&doc, 1, 0, whole(&doc)));

        let before = prefix_digest(&doc, 1, 0, whole(&doc));
        doc.layers[0].adjustment = Some(pictura_core::AdjustmentData {
            key: *b"PtFl",
            data: vec![1, 2, 3].into(),
        });
        assert_ne!(before, prefix_digest(&doc, 1, 0, whole(&doc)));
    }

    #[test]
    fn prefix_digest_ignores_edits_at_or_above() {
        let mut doc = two_layers();
        let before = prefix_digest(&doc, 1, 0, whole(&doc));
        doc.layers[1].opacity = 128;
        doc.layers[1].channels[0].data = vec![7u8; 16].into();
        assert_eq!(before, prefix_digest(&doc, 1, 0, whole(&doc)));
    }

    #[test]
    fn backdrop_plus_suffix_equals_full_composite() {
        let doc = two_layers();
        let mut session = RenderSession::default();
        let frame = session
            .preview(&doc, &["1"], 0, whole(&doc), 0)
            .expect("preview");
        assert_eq!(frame, crate::composite_rgba(&doc));
    }

    #[test]
    fn backdrop_matches_document_oracle() {
        let doc = two_layers();
        let mut session = RenderSession::default();
        let frame = session
            .preview(&doc, &["1"], 0, whole(&doc), 0)
            .expect("preview");
        assert_eq!(frame, crate::composite_rgba(&doc));
    }

    #[test]
    fn backdrop_matches_clipping_oracle() {
        let mut doc = clipped_document();
        let mut unclipped = doc.clone();
        doc.layers[1].clipping = true;
        unclipped.layers[1].clipping = false;
        let mut session = RenderSession::default();
        // A clipped target has no valid resume; the caller's exact path is the
        // oracle, so assert the fallback and that the fixture really clips.
        assert!(session.preview(&doc, &["1"], 0, whole(&doc), 0).is_none());
        assert_ne!(
            crate::composite_rgba(&doc),
            crate::composite_rgba(&unclipped),
            "the fixture must exercise clipping"
        );
    }

    #[test]
    fn backdrop_matches_knockout_oracle() {
        let mut doc = two_layers();
        doc.layers[1].knockout = Knockout::Deep;
        let mut session = RenderSession::default();
        let frame = session
            .preview(&doc, &["1"], 0, whole(&doc), 0)
            .expect("preview");
        assert_eq!(frame, crate::composite_rgba(&doc));
    }

    #[test]
    fn second_frame_reuses_the_backdrop() {
        let doc = two_layers();
        let mut session = RenderSession::default();
        let first = session
            .preview(&doc, &["1"], 0, whole(&doc), 0)
            .expect("preview");
        let second = session
            .preview(&doc, &["1"], 0, whole(&doc), 0)
            .expect("preview");
        assert_eq!(first, second);
        assert_eq!(session.backdrop_hits(), 1);
        assert_eq!(session.backdrop_builds(), 1);
    }

    #[test]
    fn level0_preview_equals_the_full_region() {
        let doc = two_layers();
        let region = rect(2, 2, 6, 6);
        let mut session = RenderSession::default();
        let frame = session
            .preview(&doc, &["1"], 0, region, 0)
            .expect("preview");
        let full = crate::composite_rgba(&doc);
        let mut want = PixelBuffer::new(4, 4, 4);
        for c in 0..4 {
            let fplane = (doc.width * doc.height) as usize;
            let rplane = 16;
            for y in 0..4 {
                let src = c * fplane + (y + 2) * doc.width as usize + 2;
                let dst = c * rplane + y * 4;
                want.data[dst..dst + 4].copy_from_slice(&full.data[src..src + 4]);
            }
        }
        assert_eq!(frame, want);
    }

    #[test]
    fn reduced_preview_need_not_equal_the_exact_level() {
        let mut doc = document(
            2048,
            2048,
            vec![layer("top", rect(0, 0, 2048, 2048), [200, 100, 50, 255])],
        );
        let mut session = RenderSession::default();
        let frame = session
            .preview(&doc, &["0"], 2, whole(&doc), 0)
            .expect("preview");
        assert_eq!((frame.width, frame.height), (512, 512));
        doc.layers[0].opacity = 128;
        let changed = session
            .preview(&doc, &["0"], 2, whole(&doc), 0)
            .expect("preview");
        assert_ne!(frame, changed);
    }

    #[test]
    fn reduced_document_rebuilds_when_the_document_changes_outside_the_preview() {
        let doc = document(
            1024,
            1024,
            vec![layer("top", rect(0, 0, 1024, 1024), [200, 100, 50, 255])],
        );
        let mut session = RenderSession::default();
        session.preview(&doc, &["0"], 2, whole(&doc), 0);
        session.preview(&doc, &["0"], 2, whole(&doc), 0);
        assert_eq!(session.preview_doc_builds(), 1);
        session.preview(&doc, &["0"], 2, whole(&doc), 1);
        assert_eq!(session.preview_doc_builds(), 2);
    }

    #[test]
    fn unreducible_content_falls_back_to_full() {
        let mut layer = layer("shape", rect(0, 0, 8, 8), [0, 0, 0, 0]);
        layer.channels.clear();
        layer.extra_blocks = vec![pictura_core::LayerBlock {
            key: *b"vscg",
            data: vec![0xFF; 3].into(),
        }];
        let doc = document(1024, 1024, vec![layer]);
        let mut session = RenderSession::default();
        assert!(session.preview(&doc, &["0"], 2, whole(&doc), 0).is_none());
    }

    #[test]
    fn empty_rect_is_a_noop() {
        let doc = two_layers();
        let mut session = RenderSession::default();
        assert!(session
            .preview(&doc, &["1"], 0, rect(0, 0, 0, 0), 0)
            .is_none());
    }

    #[test]
    fn no_presented_level_falls_back_to_exact() {
        let doc = two_layers();
        let mut session = RenderSession::default();
        assert!(session.preview(&doc, &["1"], 99, whole(&doc), 0).is_none());
    }

    #[test]
    fn clipped_target_falls_back_to_full() {
        let mut doc = two_layers();
        doc.layers[1].clipping = true;
        let mut session = RenderSession::default();
        assert!(session.preview(&doc, &["1"], 0, whole(&doc), 0).is_none());
    }

    #[test]
    fn nested_target_falls_back_to_full() {
        let doc = two_layers();
        let mut session = RenderSession::default();
        assert!(session.preview(&doc, &["0/0"], 0, whole(&doc), 0).is_none());
    }

    #[test]
    fn native_depth_falls_back_to_full() {
        let mut doc = two_layers();
        doc.depth = BitDepth::Sixteen;
        assert!(
            doc.source_depth.is_none(),
            "working depth, not a read depth"
        );
        let mut session = RenderSession::default();
        assert!(session.preview(&doc, &["1"], 0, whole(&doc), 0).is_none());
    }

    #[test]
    fn knockout_above_the_resume_point_is_exact() {
        let mut doc = two_layers();
        doc.layers[1].knockout = Knockout::Deep;
        let mut session = RenderSession::default();
        let frame = session
            .preview(&doc, &["1"], 0, whole(&doc), 0)
            .expect("preview");
        assert_eq!(frame, crate::composite_rgba(&doc));
        assert_eq!(session.backdrop_builds(), 1);
    }

    #[test]
    fn root_level_target_resumes_and_matches_full() {
        let doc = two_layers();
        let mut session = RenderSession::default();
        let frame = session
            .preview(&doc, &["1"], 0, whole(&doc), 0)
            .expect("preview");
        assert_eq!(frame, crate::composite_rgba(&doc));
    }

    #[test]
    fn over_budget_backdrop_is_refused() {
        let doc = two_layers();
        let mut session = RenderSession::with_budget(1);
        assert!(session.preview(&doc, &["1"], 0, whole(&doc), 0).is_none());
    }
}
