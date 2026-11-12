use super::*;

use super::super::super::helpers::{
    active_layer_visible, active_mask_target, active_pixel_layer, paint_mode_from, rgba_from_argb,
};
use super::super::super::helpers_composite::{crop_plane, crop_selection};
use super::super::super::PictureViewRust;
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::helpers::memory_gate as budget;
use crate::cxxqt_object::impl_core::{JobBar, Outcome};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::mask_ops::TiledMask;
use pictura_core::{Document, Layer, PsdRect};
use pictura_paint::bucket::{self, BucketPaint, BucketSource};
use pictura_paint::gradient::{self, GradientOptions, GradientStyle, PRESET_NAMES};
use pictura_paint::pattern;
use pictura_paint::Alpha;
use std::borrow::Cow;

pub(super) fn gradient_preset_count() -> i32 {
    PRESET_NAMES.len() as i32
}

pub(super) fn gradient_preset_name(index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| PRESET_NAMES.get(i))
        .map_or_else(QString::default, |name| QString::from(*name))
}

pub(super) fn gradient_preset_strip(
    index: i32,
    foreground: u32,
    background: u32,
    width: i32,
    height: i32,
) -> Vec<u8> {
    let (fg, bg) = (rgba_from_argb(foreground), rgba_from_argb(background));
    usize::try_from(index)
        .ok()
        .and_then(|i| gradient::preset(i, fg, bg))
        .map_or_else(Vec::new, |ramp| {
            ramp.preview(width.clamp(1, 4096), height.clamp(1, 4096))
                .data
                .concat()
        })
}

pub(super) fn draw_gradient(
    view: Pin<&mut PictureView>,
    preset: i32,
    foreground: u32,
    background: u32,
    style: i32,
    mode: &QString,
    opacity: i32,
    reverse: bool,
    dither: bool,
    transparency: bool,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> bool {
    // Built per drag: "Foreground to Background" means the colours as they
    // are now.
    let Some(ramp) = usize::try_from(preset)
        .ok()
        .and_then(|i| gradient::preset(i, rgba_from_argb(foreground), rgba_from_argb(background)))
    else {
        return false;
    };
    let options = GradientOptions {
        style: GradientStyle::from_i32(style).unwrap_or_default(),
        mode: paint_mode_from(&mode.to_string()),
        opacity: opacity.clamp(0, 100) as f32 / 100.0,
        reverse,
        dither,
        transparency,
    };
    let (start, end) = ((x0 as f32, y0 as f32), (x1 as f32, y1 as f32));
    apply_job(
        view,
        "Gradient",
        |doc, path, t| spill_gate(doc, path, t.bounds, 0),
        move |doc, path, t, selection| {
            // The drag is in document space; a mask's document sits at its own
            // rectangle, so its points move by the target's origin.
            let (ox, oy) = (t.origin.0 as f32, t.origin.1 as f32);
            let start = (start.0 - ox, start.1 - oy);
            let end = (end.0 - ox, end.1 - oy);
            let within = selection.map(|_| t.bounds);
            gradient::draw(doc, path, &ramp, &options, start, end, selection, within)
        },
    )
}

/// The fills' memory gate (#1300): `resident` bytes of the fill's own buffers
/// and the refresh it composites densely must fit the room; the tiles it
/// forks may spill to scratch.
fn spill_gate(doc: &Document, path: &str, bounds: PsdRect, resident: u64) -> bool {
    let cost = budget::fill_cost(doc, path, bounds, resident);
    budget::admit_spilling(cost.working, cost.spillable)
}

pub(super) fn bucket_fill_at(
    view: Pin<&mut PictureView>,
    x: i32,
    y: i32,
    foreground: u32,
    pattern: i32,
    mode: &QString,
    opacity: i32,
    tolerance: i32,
    antialias: bool,
    contiguous: bool,
    all_layers: bool,
) -> bool {
    let tile = match usize::try_from(pattern) {
        Ok(index) => match pattern::tile(index) {
            Some(tile) => Some(tile),
            None => return false,
        },
        Err(_) => None,
    };
    let foreground = rgba_from_argb(foreground);
    let mode = paint_mode_from(&mode.to_string());
    let opacity = opacity.clamp(0, 100) as f32 / 100.0;
    let tolerance = tolerance.clamp(0, 255) as u8;
    // Read on the GUI thread: the engine may run on a worker (#1300).
    let stored = view
        .rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| super::super::super::picture::stored_is_current(doc, view.rust()));
    // The flood's visited bits and its mask, then the anti-aliased copy beside
    // the mask, then the mask beside the tiles the fill forks.
    let gate = |doc: &Document, path: &str, t: &FillTarget| {
        spill_gate(doc, path, t.bounds, 3 * budget::flood_bytes(t.bounds))
    };
    apply_job(
        view,
        "Paint Bucket",
        gate,
        move |doc, path, t, selection| {
            let paint = match &tile {
                Some(tile) => BucketPaint::Pattern(tile),
                None => BucketPaint::Foreground(foreground),
            };
            let (ox, oy) = t.origin;
            let seed = (x - ox, y - oy);
            let flood = {
                // All Layers samples the composite of the layer's own document, a
                // tile at a time (#1216); a mask has no composite, so it always
                // floods its own coverage.
                let fetch = super::super::super::picture::sample_tiles_from(doc, stored);
                let source = if all_layers && !t.is_mask {
                    BucketSource::Tiles {
                        fetch: &fetch,
                        width: doc.width,
                        height: doc.height,
                    }
                } else {
                    BucketSource::layer(doc, path)?
                };
                let size = (doc.width, doc.height);
                bucket::flood(
                    source, size, t.bounds, selection, seed, tolerance, contiguous,
                )?
            };
            let mask = if antialias {
                bucket::antialias(flood, (doc.width, doc.height), t.bounds, selection)
            } else {
                flood
            };
            bucket::fill_tiled(
                doc,
                path,
                t.bounds,
                &mask,
                paint,
                mode,
                opacity,
                selection,
                Alpha::Blend,
            )
        },
    )
}

pub(super) fn edit_fill(
    view: Pin<&mut PictureView>,
    foreground: u32,
    pattern: i32,
    mode: &QString,
    opacity: i32,
    preserve_transparency: bool,
) -> bool {
    if view.rust().modal_transform_blocks_fill() {
        return false;
    }
    let tile = match usize::try_from(pattern) {
        Ok(index) => match pattern::tile(index) {
            Some(tile) => Some(tile),
            None => return false,
        },
        Err(_) => None,
    };
    let foreground = rgba_from_argb(foreground);
    let mode = paint_mode_from(&mode.to_string());
    let opacity = opacity.clamp(0, 100) as f32 / 100.0;
    // Preserve Transparency is the engine's own policy: it keeps every filled
    // pixel's alpha and leaves the layer's empty pixels alone. A mask's
    // transparency lock already does that, so the policy is a no-op on a mask.
    let alpha = if preserve_transparency {
        Alpha::Keep
    } else {
        Alpha::Blend
    };
    apply_job(
        view,
        "Fill",
        |doc, path, t| spill_gate(doc, path, t.bounds, 0),
        move |doc, path, t, selection| {
            let paint = match &tile {
                Some(tile) => BucketPaint::Pattern(tile),
                None => BucketPaint::Foreground(foreground),
            };
            bucket::fill_within(
                doc, path, t.bounds, None, paint, mode, opacity, selection, alpha,
            )
        },
    )
}

pub(super) fn edit_stroke(
    view: Pin<&mut PictureView>,
    color: u32,
    width: i32,
    position: i32,
    mode: &QString,
    opacity: i32,
) -> bool {
    if view.rust().modal_transform_blocks_fill() {
        return false;
    }
    if view.rust().session.selection.is_none() {
        return false;
    }
    let align = match position {
        0 => bucket::StrokeAlign::Inside,
        2 => bucket::StrokeAlign::Outside,
        _ => bucket::StrokeAlign::Center,
    };
    let width = width.clamp(0, 250) as u32;
    let color = rgba_from_argb(color);
    let mode = paint_mode_from(&mode.to_string());
    let opacity = opacity.clamp(0, 100) as f32 / 100.0;
    // The dilation, erosion and band of the window's edge tiles, beside the
    // tiles the band's fill forks.
    let gate = move |doc: &Document, path: &str, t: &FillTarget| {
        let canvas = (doc.width as i32, doc.height as i32);
        bucket::stroke_window(t.bounds, width as i32 + 1, canvas)
            .is_some_and(|window| spill_gate(doc, path, window, 3 * budget::flood_bytes(window)))
    };
    apply_job(view, "Stroke", gate, move |doc, path, t, selection| {
        bucket::stroke_selection(
            doc, path, selection?, t.bounds, color, width, align, mode, opacity,
        )
    })
}

/// Where a shared-path fill lands: the active pixel layer inside the document,
/// or the active mask as a mask-sized document of its own. A mask's document
/// holds the coverage in one grayscale channel and sits at the mask's
/// rectangle, so a document-space point maps to `point - origin` and a
/// document-space coverage is confined with [`Self::coverage`].
pub(crate) struct FillTarget {
    /// The target's top-left corner in the document's coordinates.
    pub origin: (i32, i32),
    /// The document's size, for a fill that builds a document-space coverage.
    pub document_size: (u32, u32),
    /// The target's rectangle in the document's coordinates.
    pub rect: PsdRect,
    /// Whether the target is a mask, so geometry is translated and a
    /// document-space coverage is confined to the mask's rectangle.
    pub is_mask: bool,
    /// What the edit can change, in the edited document's coordinates: the
    /// selection's box, or the whole target without a selection (#1207).
    pub bounds: PsdRect,
}

impl FillTarget {
    /// A document-space coverage as a coverage of the edited document:
    /// unchanged for a layer, cropped to the mask's rectangle for a mask.
    pub(crate) fn coverage<'a>(&self, doc_space: &'a [u8]) -> Cow<'a, [u8]> {
        if self.is_mask {
            Cow::Owned(crop_selection(
                doc_space,
                self.document_size.0,
                self.document_size.1,
                self.rect,
            ))
        } else {
            Cow::Borrowed(doc_space)
        }
    }
}

/// Run `edit` on a copy of the document and its visible lone active pixel
/// layer, or — with a mask edit target — on the mask's coverage; when it
/// changed something, adopt the result and record it as `label`. `selection` is
/// the selection cropped to the target. False (no state) mid-stroke, without
/// such a target, or when nothing changed.
pub(crate) fn apply(
    mut view: Pin<&mut PictureView>,
    label: &str,
    edit: impl FnOnce(&mut Document, &str, &FillTarget, Option<&TiledMask>) -> Option<PsdRect>,
) -> bool {
    budget::clear();
    if blocked(view.rust()) {
        return false;
    }
    if view.rust().session.quick_mask.active {
        if view.rust().layer_target.is_empty() || view.rust().session.live.kind().is_some() {
            return false;
        }
        let Some(mut document) = view.rust().quick_mask_document(true) else {
            return false;
        };
        let rect = document.layers[0].rect;
        let clip = view.rust().session.selection.as_ref();
        let bounds = match clip {
            Some(selection) => match selection.bounds().and_then(|b| b.intersect(rect)) {
                Some(bounds) => bounds,
                None => return false,
            },
            None => rect,
        };
        let target = FillTarget {
            origin: (0, 0),
            document_size: (document.width, document.height),
            rect,
            is_mask: true,
            bounds,
        };
        if edit(&mut document, "0", &target, clip.map(|s| &s.data)).is_none() {
            return false;
        }
        let changed = view
            .as_mut()
            .rust_mut()
            .adopt_quick_mask_document(document, true);
        if changed {
            view.as_mut().edit(label, |_| Edit::Meta);
        }
        return changed;
    }
    if let Some(path) = active_mask_target(view.rust()) {
        return apply_mask(view, label, &path, edit);
    }
    let edited = {
        let rust = view.rust();
        let (Some(doc), Some((path, target))) = (rust.session.doc.as_ref(), layer_fill(rust))
        else {
            return false;
        };
        let selection = rust.session.selection.as_ref().map(|s| &s.data);
        let mut edited = doc.clone();
        edit(&mut edited, &path, &target, selection).map(|rect| (edited, rect))
    };
    let Some((doc, rect)) = edited else {
        return false;
    };
    view.as_mut().rust_mut().session.doc = Some(doc);
    refresh_edit(view.as_mut(), Some(rect), "");
    view.as_mut().record(label);
    true
}

/// Whether no fill may run now: mid-stroke, or under a modal transform.
fn blocked(rust: &PictureViewRust) -> bool {
    rust.session.live.stroke().is_some() || rust.modal_transform_blocks_edit()
}

/// The visible lone active pixel layer a fill writes, and its target: the
/// whole canvas, or the selection's box on it. `None` without such a layer or
/// for a selection that covers nothing.
fn layer_fill(rust: &PictureViewRust) -> Option<(String, FillTarget)> {
    let doc = rust.session.doc.as_ref()?;
    let path = rust.layer_target.single()?;
    if active_pixel_layer(doc, Some(path)).is_none() || !active_layer_visible(doc, Some(path)) {
        return None;
    }
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    let bounds = match rust.session.selection {
        Some(_) => rust
            .selection_mask_bounds()
            .and_then(|b| b.intersect(canvas))?,
        None => canvas,
    };
    let target = FillTarget {
        origin: (0, 0),
        document_size: (doc.width, doc.height),
        rect: canvas,
        is_mask: false,
        bounds,
    };
    Some((path.to_string(), target))
}

/// [`apply`] for a fill the shell can run as a job (#1300). `gate` runs on the
/// GUI thread before anything changes, where the memory gate records its
/// refusal. On an armed view and a large document, a pixel-layer fill then
/// runs `edit` on a worker as a document job that lands as one `label` state;
/// a mask or Quick Mask target, and any other call, runs at once through
/// [`apply`]. Either way the outcome is noted for the shell's job window.
pub(super) fn apply_job(
    mut view: Pin<&mut PictureView>,
    label: &str,
    gate: impl FnOnce(&Document, &str, &FillTarget) -> bool,
    edit: impl FnOnce(&mut Document, &str, &FillTarget, Option<&TiledMask>) -> Option<PsdRect>
        + Send
        + 'static,
) -> bool {
    let job = {
        let rust = view.rust();
        rust.document_job_wanted()
            && !blocked(rust)
            && !rust.session.quick_mask.active
            && active_mask_target(rust).is_none()
    };
    if !job {
        let applied = apply(view.as_mut(), label, |doc, path, t, selection| {
            if !gate(doc, path, t) {
                return None;
            }
            edit(doc, path, t, selection)
        });
        let outcome = applied.then(Outcome::default);
        return view.as_mut().rust_mut().note_document_op(outcome);
    }
    budget::clear();
    let prepared = {
        let rust = view.rust();
        layer_fill(rust).filter(|(path, target)| {
            rust.session
                .doc
                .as_ref()
                .is_some_and(|doc| gate(doc, path, target))
        })
    };
    let Some((path, target)) = prepared else {
        return view.as_mut().rust_mut().note_document_op(None);
    };
    // A plane clone shares its tiles: the worker reads the selection as it is.
    let selection = view
        .rust()
        .session
        .selection
        .as_ref()
        .map(|s| s.data.clone());
    view.document_op(
        label,
        JobBar::Busy,
        move |doc| {
            let rect = edit(doc, &path, &target, selection.as_ref())?;
            Some(Outcome {
                rect: Some(rect),
                ..Outcome::default()
            })
        },
        |rust, outcome| fill_edit(rust, outcome.rect),
    )
}

/// The edit a landed fill job records: its rect, or the whole document when a
/// layer style could carry the change past it, as [`refresh_edit`] decides.
fn fill_edit(rust: &mut PictureViewRust, rect: Option<PsdRect>) -> Edit {
    let styled = rust
        .session
        .doc
        .as_ref()
        .is_none_or(|doc| styled_except(&doc.layers, "", ""));
    match rect {
        Some(rect) if !styled => Edit::regions(vec![rect], true),
        _ => Edit::Full,
    }
}

/// [`apply`] against the mask at `path`: the fill runs on the mask's coverage
/// and is written back to the real mask, recorded as one `label` state. A fill
/// colour and a ramp are reduced to their luma by the grayscale document.
pub(super) fn apply_mask(
    mut view: Pin<&mut PictureView>,
    label: &str,
    path: &str,
    edit: impl FnOnce(&mut Document, &str, &FillTarget, Option<&TiledMask>) -> Option<PsdRect>,
) -> bool {
    let outcome = view.as_mut().edit(label, |rust| {
        let Some(doc) = rust.session.doc.as_ref() else {
            return Edit::None;
        };
        let Some(mask) = pictura_render::mask_document_coverage(doc, path) else {
            return Edit::None;
        };
        let (ox, oy) = (mask.rect.left, mask.rect.top);
        let doc_bounds = match rust.session.selection {
            Some(_) => match rust
                .selection_mask_bounds()
                .and_then(|b| b.intersect(mask.rect))
            {
                Some(bounds) => bounds,
                None => return Edit::None,
            },
            None => mask.rect,
        };
        let target = FillTarget {
            origin: (ox, oy),
            document_size: (doc.width, doc.height),
            rect: mask.rect,
            is_mask: true,
            bounds: PsdRect {
                top: doc_bounds.top - oy,
                left: doc_bounds.left - ox,
                bottom: doc_bounds.bottom - oy,
                right: doc_bounds.right - ox,
            },
        };
        let selection = rust
            .session
            .selection
            .as_ref()
            .map(|s| TiledMask::from(crop_plane(&s.data, s.width, s.height, mask.rect)));
        let mut document = mask.document;
        let Some(dirty) = edit(&mut document, "0", &target, selection.as_ref()) else {
            return Edit::None;
        };
        let Some(real) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        if !pictura_render::write_mask_back(real, path, &document, Some(dirty)) {
            return Edit::None;
        }
        let rect = PsdRect {
            top: dirty.top + mask.rect.top,
            left: dirty.left + mask.rect.left,
            bottom: dirty.bottom + mask.rect.top,
            right: dirty.right + mask.rect.left,
        };
        Edit::regions(vec![rect], false)
    });
    !matches!(outcome, Edit::None)
}

/// Present an edit that changed only `rect` of the document: composite that
/// region, not the document, then emit `changed` for the panels. A layer style
/// could carry the change past `rect` (a shadow, a glow), so with one anywhere
/// but on `own` (a layer whose style `rect` already covers), or with no `rect`,
/// the whole document is recomposited.
pub(crate) fn refresh_edit(mut view: Pin<&mut PictureView>, rect: Option<PsdRect>, own: &str) {
    let styled = view
        .rust()
        .session
        .doc
        .as_ref()
        .is_none_or(|doc| styled_except(&doc.layers, "", own));
    match rect {
        Some(rect) if !styled => {
            view.as_mut().refresh_regions(&[rect]);
            view.as_mut().changed();
        }
        _ => view.as_mut().recomposite(),
    }
}

/// Whether a layer under `prefix` other than the one at path `own` carries a
/// layer style.
pub(super) fn styled_except(layers: &[Layer], prefix: &str, own: &str) -> bool {
    layers.iter().enumerate().any(|(i, l)| {
        let path = if prefix.is_empty() {
            i.to_string()
        } else {
            format!("{prefix}/{i}")
        };
        (path != own && pictura_render::has_layer_style(l))
            || styled_except(&l.children, &path, own)
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::super::tests::pixel_layer;
    use super::styled_except;
    use pictura_core::{Layer, LayerBlock};

    fn styled() -> Layer {
        let mut layer = pixel_layer("styled", 1, 1, (0, 0, 0));
        layer.extra_blocks.push(LayerBlock {
            key: *b"lfx2",
            data: Vec::new().into(),
        });
        layer
    }

    #[test]
    fn a_style_anywhere_but_the_exempt_layer_counts() {
        let plain = pixel_layer("plain", 1, 1, (0, 0, 0));
        let mut group = pixel_layer("group", 1, 1, (0, 0, 0));
        group.is_group = true;
        group.children = pictura_core::LayerStack::from_layers(vec![plain.clone(), styled()]);
        let layers = vec![plain.clone(), group];
        assert!(styled_except(&layers, "", ""), "a nested style is found");
        assert!(
            !styled_except(&layers, "", "1/1"),
            "the exempt layer's own style"
        );
        assert!(
            styled_except(&layers, "", "1"),
            "exempting the group spares not its child"
        );
        assert!(!styled_except(&[plain], "", ""));
    }
}
