//! The fill tool bridges: the Gradient and the Paint Bucket. Free functions
//! over a [`PictureView`], as in the parent module. One gesture is one edit,
//! so each applies it through the selection, recomposites, and records its own
//! history state (`"Gradient"`, `"Paint Bucket"`). The shape tools' Fill
//! Pixels mode shares [`apply`].
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::helpers::{
    active_layer_visible, active_mask_target, active_pixel_layer, luma_u8, paint_mode_from,
    rgba_from_argb,
};
use super::super::helpers_composite::{crop_selection, current_buffer};
use super::super::qobject::PictureView;
use super::super::PictureViewRust;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{Document, Layer, PsdRect};
use pictura_paint::bucket::{self, BucketPaint};
use pictura_paint::eraser::antialias_mask;
use pictura_paint::gradient::{self, GradientOptions, GradientStyle, PRESET_NAMES};
use pictura_paint::healing::RgbaImage;
use pictura_paint::pattern;
use pictura_paint::stamp::{layer_surface, surface_from_composite};
use pictura_paint::{PaintMode, Rgba};

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
    apply(view, "Gradient", |doc, path, rust| {
        gradient::draw(doc, path, &ramp, &options, start, end, selection(rust))
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
    apply(view, "Paint Bucket", |doc, path, rust| {
        let surface = if all_layers {
            surface_from_composite(&current_buffer(doc, rust.gpu_compute))
        } else {
            layer_surface(doc, path)?
        };
        let flood = bucket::flood(&surface, (x, y), tolerance, contiguous)?;
        let mask = if antialias {
            antialias_mask(&flood, surface.width as usize)
        } else {
            flood
        };
        bucket::fill(doc, path, &mask, paint, mode, opacity, selection(rust))
    })
}

pub(crate) fn selection(rust: &PictureViewRust) -> Option<&[u8]> {
    rust.selection.as_ref().map(|s| s.data.as_slice())
}

fn edit_fill(
    view: Pin<&mut PictureView>,
    foreground: u32,
    pattern: i32,
    mode: &QString,
    opacity: i32,
    preserve_transparency: bool,
) -> bool {
    let tile = match usize::try_from(pattern) {
        Ok(index) => match pattern::tile(index) {
            Some(tile) => Some(tile),
            None => return false,
        },
        Err(_) => None,
    };
    let mode = paint_mode_from(&mode.to_string());
    let opacity = opacity.clamp(0, 100) as f32 / 100.0;
    // A mask target fills the mask's coverage with the fill colour's luma.
    if let Some(path) = active_mask_target(view.rust()) {
        return fill_mask(view, &path, tile.as_ref(), foreground, mode, opacity);
    }
    let paint = match &tile {
        Some(tile) => BucketPaint::Pattern(tile),
        None => BucketPaint::Foreground(rgba_from_argb(foreground)),
    };
    apply(view, "Fill", |doc, path, rust| {
        let (width, height) = (doc.width as i32, doc.height as i32);
        // A whole-layer fill is a document-sized mask of full coverage; Preserve
        // Transparency narrows it to the pixels the layer already has.
        let mask = if preserve_transparency {
            let surface = layer_surface(doc, path)?;
            surface
                .data
                .iter()
                .map(|px| if px[3] > 0 { 255 } else { 0 })
                .collect::<Vec<u8>>()
        } else {
            vec![255u8; (width * height) as usize]
        };
        bucket::fill(doc, path, &mask, paint, mode, opacity, selection(rust))
    })
}

/// `Edit ▸ Fill` on the mask of the layer at `path`: the fill colour's luma
/// (a pattern's per-pixel luma) fills the coverage through the selection, which
/// is cropped to the mask rectangle. One "Fill" state.
fn fill_mask(
    mut view: Pin<&mut PictureView>,
    path: &str,
    tile: Option<&RgbaImage>,
    foreground: u32,
    mode: PaintMode,
    opacity: f32,
) -> bool {
    let (mut mask, selection) = {
        let rust = view.rust();
        let Some(doc) = rust.doc.as_ref() else {
            return false;
        };
        let Some(mask) = pictura_render::mask_document(doc, path) else {
            return false;
        };
        let selection = rust
            .selection
            .as_ref()
            .map(|s| crop_selection(&s.data, s.width, s.height, mask.rect));
        (mask, selection)
    };
    let gray_tile;
    let paint = match tile {
        Some(tile) => {
            gray_tile = gray_image(tile);
            BucketPaint::Pattern(&gray_tile)
        }
        None => {
            let color = rgba_from_argb(foreground);
            let v = luma_u8(color);
            BucketPaint::Foreground(Rgba {
                r: v,
                g: v,
                b: v,
                a: color.a,
            })
        }
    };
    let full = vec![255u8; (mask.document.width * mask.document.height) as usize];
    let Some(dirty) = bucket::fill(
        &mut mask.document,
        "0",
        &full,
        paint,
        mode,
        opacity,
        selection.as_deref(),
    ) else {
        return false;
    };
    let changed = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.doc.as_mut() else {
            return false;
        };
        pictura_render::write_mask_back(doc, path, &mask.document, Some(dirty))
    };
    if changed {
        let rect = PsdRect {
            top: dirty.top + mask.rect.top,
            left: dirty.left + mask.rect.left,
            bottom: dirty.bottom + mask.rect.top,
            right: dirty.right + mask.rect.left,
        };
        view.as_mut().refresh_region(rect);
        view.as_mut().record("Fill");
    }
    changed
}

/// A pattern tile as its per-pixel luma, so a pattern fill of a mask paints
/// coverage rather than colour.
fn gray_image(tile: &RgbaImage) -> RgbaImage {
    RgbaImage {
        width: tile.width,
        height: tile.height,
        data: tile
            .data
            .iter()
            .map(|px| {
                let v = luma_u8(Rgba {
                    r: px[0],
                    g: px[1],
                    b: px[2],
                    a: px[3],
                });
                [v, v, v, 255]
            })
            .collect(),
    }
}

fn edit_stroke(
    view: Pin<&mut PictureView>,
    color: u32,
    width: i32,
    position: i32,
    mode: &QString,
    opacity: i32,
) -> bool {
    let Some(selection) = view.rust().selection.as_ref() else {
        return false;
    };
    let selection = selection.data.clone();
    let align = match position {
        0 => bucket::StrokeAlign::Inside,
        2 => bucket::StrokeAlign::Outside,
        _ => bucket::StrokeAlign::Center,
    };
    apply(view, "Stroke", |doc, path, _| {
        bucket::stroke_selection(
            doc,
            path,
            &selection,
            rgba_from_argb(color),
            width.clamp(0, 250) as u32,
            align,
            paint_mode_from(&mode.to_string()),
            opacity.clamp(0, 100) as f32 / 100.0,
        )
    })
}

/// Run `edit` on a copy of the document and its visible lone active pixel
/// layer; when it changed something, adopt the copy and record it as `label`.
/// False (no state) mid-stroke, without such a layer, or when nothing changed.
pub(crate) fn apply(
    mut view: Pin<&mut PictureView>,
    label: &str,
    edit: impl FnOnce(&mut Document, &str, &PictureViewRust) -> Option<PsdRect>,
) -> bool {
    let edited = {
        let rust = view.rust();
        let (Some(doc), Some(path)) = (rust.doc.as_ref(), rust.active_layer.as_deref()) else {
            return false;
        };
        // ponytail: of the fills only Edit > Fill writes a mask (`fill_mask`);
        // the rest refuse a mask target rather than write the layer's pixels.
        if rust.stroke.is_some()
            || active_mask_target(rust).is_some()
            || active_pixel_layer(doc, Some(path)).is_none()
            || !active_layer_visible(doc, Some(path))
        {
            return false;
        }
        let mut edited = doc.clone();
        edit(&mut edited, path, rust).map(|rect| (edited, rect))
    };
    let Some((doc, rect)) = edited else {
        return false;
    };
    view.as_mut().rust_mut().doc = Some(doc);
    refresh_edit(view.as_mut(), Some(rect), "");
    view.as_mut().record(label);
    true
}

/// Present an edit that changed only `rect` of the document: composite that
/// region, not the document, then emit `changed` for the panels. A layer style
/// could carry the change past `rect` (a shadow, a glow), so with one anywhere
/// but on `own` (a layer whose style `rect` already covers), or with no `rect`,
/// the whole document is recomposited.
pub(crate) fn refresh_edit(mut view: Pin<&mut PictureView>, rect: Option<PsdRect>, own: &str) {
    let styled = view
        .rust()
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
            data: Vec::new(),
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
