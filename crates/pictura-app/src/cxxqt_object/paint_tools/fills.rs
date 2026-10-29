//! The fill tool bridges: the Gradient and the Paint Bucket. Free functions
//! over a [`PictureView`], as in the parent module. One gesture is one edit,
//! so each applies it through the selection, recomposites, and records its own
//! history state (`"Gradient"`, `"Paint Bucket"`). The shape tools' Fill
//! Pixels mode shares [`apply`].
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::helpers::{
    active_layer_visible, active_mask_target, active_pixel_layer, paint_mode_from, rgba_from_argb,
};
use super::super::helpers_composite::crop_selection;
use super::super::picture::picture;
use super::super::qobject::PictureView;
use super::super::PictureViewRust;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{Document, Layer, PsdRect};
use pictura_paint::bucket::{self, BucketPaint};
use pictura_paint::eraser::antialias_mask;
use pictura_paint::gradient::{self, GradientOptions, GradientStyle, PRESET_NAMES};
use pictura_paint::pattern;
use pictura_paint::stamp::{layer_surface, surface_from_composite};
use pictura_paint::Alpha;
use std::borrow::Cow;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The number of built-in gradients.
        fn gradient_preset_count() -> i32;

        /// Built-in gradient `index`'s name, or empty out of range.
        fn gradient_preset_name(index: i32) -> QString;

        /// Built-in gradient `index` between `foreground` and `background`
        /// (`0xAARRGGBB`) as a horizontal strip, packed straight RGBA8888
        /// `width` × `height`; empty out of range.
        fn gradient_preset_strip(
            index: i32,
            foreground: u32,
            background: u32,
            width: i32,
            height: i32,
        ) -> Vec<u8>;

        /// Gradient tool: draw built-in gradient `preset` over the active
        /// pixel layer along the drag from `(x0, y0)` to `(x1, y1)` (document
        /// space), inside the selection when there is one. `style` 0 Linear /
        /// 1 Radial / 2 Angle / 3 Reflected / 4 Diamond; `mode` is a Brush mode
        /// (`"normal"`, …); `opacity` 0–100 %; `transparency` honours the
        /// gradient's own alpha. One "Gradient" state; false when nothing
        /// changed (a drag with no length), without a visible lone pixel
        /// layer, when its pixels are locked, or mid-stroke.
        fn draw_gradient(
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
        ) -> bool;

        /// Paint Bucket: fill the pixels that match the one at document point
        /// `(x, y)` — `tolerance` 0–255 per channel, `contiguous`, read from
        /// the active layer or with `all_layers` the composite — with
        /// `foreground` (`0xAARRGGBB`), or built-in pattern `pattern` when it
        /// is not negative, at `opacity` 0–100 % in Brush mode `mode`, with a
        /// softened edge when `antialias`, inside the selection when there is
        /// one. One "Paint Bucket" state; false as for `draw_gradient`, for a
        /// point off the canvas, or for an unknown pattern.
        fn bucket_fill_at(
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
        ) -> bool;

        /// `Edit ▸ Fill…`: fill the active pixel layer (its selection when there
        /// is one) with `foreground` (`0xAARRGGBB`), or built-in pattern
        /// `pattern` when it is not negative, at `opacity` 0–100 % in Brush mode
        /// `mode`. `preserve_transparency` confines the fill to pixels that are
        /// already opaque. One "Fill" state; false without a visible lone pixel
        /// layer, when its pixels are locked, or when nothing changed.
        fn edit_fill(
            view: Pin<&mut PictureView>,
            foreground: u32,
            pattern: i32,
            mode: &QString,
            opacity: i32,
            preserve_transparency: bool,
        ) -> bool;

        /// `Edit ▸ Stroke…`: outline the active selection on the active pixel
        /// layer with a solid `color` (`0xAARRGGBB`) band `width` px wide,
        /// `position` 0 Inside / 1 Center / 2 Outside, at `opacity` 0–100 % in
        /// Brush mode `mode`. One "Stroke" state; false without a pixel
        /// selection, without a visible lone pixel layer, or when nothing
        /// changed.
        fn edit_stroke(
            view: Pin<&mut PictureView>,
            color: u32,
            width: i32,
            position: i32,
            mode: &QString,
            opacity: i32,
        ) -> bool;
    }
}

fn gradient_preset_count() -> i32 {
    PRESET_NAMES.len() as i32
}

fn gradient_preset_name(index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| PRESET_NAMES.get(i))
        .map_or_else(QString::default, |name| QString::from(*name))
}

fn gradient_preset_strip(
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

fn draw_gradient(
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
    apply(view, "Gradient", |doc, path, _rust, t, selection| {
        // The drag is in document space; a mask's document sits at its own
        // rectangle, so its points move by the target's origin.
        let (ox, oy) = (t.origin.0 as f32, t.origin.1 as f32);
        let start = (start.0 - ox, start.1 - oy);
        let end = (end.0 - ox, end.1 - oy);
        gradient::draw(doc, path, &ramp, &options, start, end, selection)
    })
}

fn bucket_fill_at(
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
    let paint = match &tile {
        Some(tile) => BucketPaint::Pattern(tile),
        None => BucketPaint::Foreground(rgba_from_argb(foreground)),
    };
    let mode = paint_mode_from(&mode.to_string());
    let opacity = opacity.clamp(0, 100) as f32 / 100.0;
    let tolerance = tolerance.clamp(0, 255) as u8;
    apply(view, "Paint Bucket", |doc, path, rust, t, selection| {
        // All Layers samples the composite of the layer's own document; a mask
        // has no composite, so it always floods its own coverage.
        let surface = if all_layers && !t.is_mask {
            surface_from_composite(&picture(doc, rust))
        } else {
            layer_surface(doc, path)?
        };
        let (ox, oy) = t.origin;
        let flood = bucket::flood(&surface, (x - ox, y - oy), tolerance, contiguous)?;
        let mask = if antialias {
            antialias_mask(&flood, surface.width as usize)
        } else {
            flood
        };
        bucket::fill(
            doc,
            path,
            &mask,
            paint,
            mode,
            opacity,
            selection,
            Alpha::Blend,
        )
    })
}

fn edit_fill(
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
    let mode = paint_mode_from(&mode.to_string());
    let opacity = opacity.clamp(0, 100) as f32 / 100.0;
    let paint = match &tile {
        Some(tile) => BucketPaint::Pattern(tile),
        None => BucketPaint::Foreground(rgba_from_argb(foreground)),
    };
    // Preserve Transparency is the engine's own policy: it keeps every filled
    // pixel's alpha and leaves the layer's empty pixels alone. A mask's
    // transparency lock already does that, so the policy is a no-op on a mask.
    let alpha = if preserve_transparency {
        Alpha::Keep
    } else {
        Alpha::Blend
    };
    apply(view, "Fill", |doc, path, _rust, _t, selection| {
        let mask = vec![255u8; (doc.width * doc.height) as usize];
        bucket::fill(doc, path, &mask, paint, mode, opacity, selection, alpha)
    })
}

fn edit_stroke(
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
    apply(view, "Stroke", |doc, path, _rust, _t, selection| {
        bucket::stroke_selection(
            doc,
            path,
            selection?,
            rgba_from_argb(color),
            width.clamp(0, 250) as u32,
            align,
            paint_mode_from(&mode.to_string()),
            opacity.clamp(0, 100) as f32 / 100.0,
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
    edit: impl FnOnce(
        &mut Document,
        &str,
        &PictureViewRust,
        &FillTarget,
        Option<&[u8]>,
    ) -> Option<PsdRect>,
) -> bool {
    if view.rust().session.live.stroke().is_some() || view.rust().modal_transform_blocks_edit() {
        return false;
    }
    if let Some(path) = active_mask_target(view.rust()) {
        return apply_mask(view, label, &path, edit);
    }
    let edited = {
        let rust = view.rust();
        let (Some(doc), Some(path)) = (rust.session.doc.as_ref(), rust.layer_target.single())
        else {
            return false;
        };
        if active_pixel_layer(doc, Some(path)).is_none() || !active_layer_visible(doc, Some(path)) {
            return false;
        }
        let target = FillTarget {
            origin: (0, 0),
            document_size: (doc.width, doc.height),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: doc.height as i32,
                right: doc.width as i32,
            },
            is_mask: false,
        };
        let selection = rust.session.selection.as_ref().map(|s| s.data.as_slice());
        let mut edited = doc.clone();
        edit(&mut edited, path, rust, &target, selection).map(|rect| (edited, rect))
    };
    let Some((doc, rect)) = edited else {
        return false;
    };
    view.as_mut().rust_mut().session.doc = Some(doc);
    refresh_edit(view.as_mut(), Some(rect), "");
    view.as_mut().record(label);
    true
}

/// [`apply`] against the mask at `path`: the fill runs on the mask's coverage
/// and is written back to the real mask, recorded as one `label` state. A fill
/// colour and a ramp are reduced to their luma by the grayscale document.
fn apply_mask(
    mut view: Pin<&mut PictureView>,
    label: &str,
    path: &str,
    edit: impl FnOnce(
        &mut Document,
        &str,
        &PictureViewRust,
        &FillTarget,
        Option<&[u8]>,
    ) -> Option<PsdRect>,
) -> bool {
    let outcome = view.as_mut().edit(label, |rust| {
        let Some(doc) = rust.session.doc.as_ref() else {
            return Edit::None;
        };
        let Some(mask) = pictura_render::mask_document_coverage(doc, path) else {
            return Edit::None;
        };
        let target = FillTarget {
            origin: (mask.rect.left, mask.rect.top),
            document_size: (doc.width, doc.height),
            rect: mask.rect,
            is_mask: true,
        };
        let selection = rust
            .session
            .selection
            .as_ref()
            .map(|s| crop_selection(&s.data, s.width, s.height, mask.rect));
        let mut document = mask.document;
        let Some(dirty) = edit(&mut document, "0", rust, &target, selection.as_deref()) else {
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
fn styled_except(layers: &[Layer], prefix: &str, own: &str) -> bool {
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
    use super::super::super::tests::pixel_layer;
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
