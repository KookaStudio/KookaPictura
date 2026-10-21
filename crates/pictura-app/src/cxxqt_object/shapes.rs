//! The shape tools (Rectangle, Rounded Rectangle, Ellipse, Polygon): the live
//! outline and the three ways a shape lands, CS6's Shape / Path / Fill Pixels
//! modes, plus the Layers panel's view of a shape layer. Free functions over a
//! [`PictureView`] (their own bridge, so the `PictureView` declaration list
//! does not grow).
//!
//! Every mode draws one [`ShapeSpec`]: a drag, or (`boxed`) the box a Create
//! dialog placed. The preview makes the same call, and each landing records
//! one `"<Tool> Tool"` state (e.g. `"Rectangle Tool"`). A Rectangle, Rounded
//! Rectangle, or Ellipse drawn in Shape mode is a live shape (a CC feature):
//! the layer keeps its parametric origin until an edit turns it into a regular
//! path (`paths.rs`).
//!
//! [`PictureView`]: super::qobject::PictureView

use super::helpers::{paint_timing, rgba_from_argb};
use super::helpers_composite::{aspect_thumbnail_extent, rgba_image};
use super::paint_tools::fills::{apply, refresh_edit, selection};
use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString};
use ffi::ShapeSpec;
use pictura_codec::LiveShape;
use pictura_core::path::{Subpath, VectorPath};
use pictura_core::shape::{self, Arrowheads, ShapeKind, ShapeOptions, CUSTOM_SHAPE_NAMES};
use pictura_core::{Layer, PsdRect};
use pictura_paint::bucket::{self, BucketPaint};
use pictura_paint::PaintMode;

#[cxx_qt::bridge]
pub mod ffi {
    /// One shape: tool `kind` (0 Rectangle, 1 Rounded Rectangle, 2 Ellipse,
    /// 3 Polygon) and either a drag from `(x0, y0)` to `(x1, y1)` (`shift`
    /// squares off or snaps the Polygon's turn to 15°, `alt` grows from the
    /// press) or, when `boxed`, the box with those corners. The Rounded
    /// Rectangle's corner radii `r_tl`, `r_tr`, `r_br`, `r_bl` (px); the
    /// Polygon's `sides`, `star` with `indent` (% of the radius), and smooth
    /// corners / indents; the Line's `weight` (px) and arrowheads (at the start
    /// / end, width and length in % of the weight, concavity %); the Custom
    /// Shape's index (kind 4 Line, 5 Custom Shape); `align_edges` snaps
    /// straight edges to the pixel grid. A Shape-mode layer is filled unless
    /// `no_fill`, and stroked when `stroke` (`stroke_color` `0xAARRGGBB`,
    /// `stroke_width` px, `stroke_align` 0 inside / 1 centre / 2 outside).
    #[namespace = "pictura"]
    struct ShapeSpec {
        kind: i32,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        shift: bool,
        alt: bool,
        boxed: bool,
        r_tl: f64,
        r_tr: f64,
        r_br: f64,
        r_bl: f64,
        sides: i32,
        star: bool,
        indent: f64,
        smooth_corners: bool,
        smooth_indents: bool,
        weight: f64,
        arrow_start: bool,
        arrow_end: bool,
        arrow_width: f64,
        arrow_length: f64,
        arrow_concavity: f64,
        custom: i32,
        align_edges: bool,
        no_fill: bool,
        stroke: bool,
        stroke_color: u32,
        stroke_width: f64,
        stroke_align: i32,
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The closed outline `spec` draws. Six numbers per anchor `[ax, ay, ix, iy, ox, oy]`, a missing handle at its anchor; empty when the shape encloses nothing.
        fn shape_outline(spec: &ShapeSpec) -> Vec<f64>;

        /// Shape mode: a new shape layer above the active layer (a `foreground` (`0xAARRGGBB`) color fill cut to the outline by a vector mask), named after the tool, live unless a Polygon. Returns its path; empty (no state) without a document or for an empty outline.
        fn shape_add_layer(
            view: Pin<&mut PictureView>,
            spec: &ShapeSpec,
            foreground: u32,
        ) -> QString;

        /// Path mode: add the outline to the Work Path as a new closed component. False (no state) without a document or for an empty outline.
        fn shape_add_path(view: Pin<&mut PictureView>, spec: &ShapeSpec) -> bool;

        /// Fill Pixels mode: paint the anti-aliased outline in `foreground` (`0xAARRGGBB`) onto the active pixel layer, inside the selection when there is one. False (no state) as for `draw_gradient`, or for an empty outline.
        fn shape_fill_pixels(
            view: Pin<&mut PictureView>,
            spec: &ShapeSpec,
            foreground: u32,
        ) -> bool;

        /// The number of built-in custom shapes.
        fn shape_custom_count() -> i32;

        /// Built-in custom shape `index`'s name, or empty out of range.
        fn shape_custom_name(index: i32) -> QString;

        /// Custom shape `index` as a black silhouette on transparency in a `size` square, for the picker; null out of range.
        fn shape_custom_preview(index: i32, size: i32) -> QImage;

        /// The active shape layer as `[width, height, fill_on, fill_argb, stroke_on, stroke_argb, stroke_width, stroke_align]` (flags 0/1, colours `0xAARRGGBB`); empty when the active layer is not a shape layer.
        fn shape_active(view: &PictureView) -> Vec<f64>;

        /// Fill the active shape layer with `argb`, or leave it unfilled (`on` false); one "Change Shape Fill" state. False (no state) when there is no active shape layer or nothing changed.
        fn shape_set_active_fill(view: Pin<&mut PictureView>, on: bool, argb: u32) -> bool;

        /// Stroke the active shape layer (`width` px, `align` 0 inside / 1 centre / 2 outside), or remove its stroke; one "Change Shape Stroke" state. False (no state) as for `shape_set_active_fill`.
        fn shape_set_active_stroke(
            view: Pin<&mut PictureView>,
            on: bool,
            argb: u32,
            width: f64,
            align: i32,
        ) -> bool;

        /// Scale the active shape layer's outline about its top-left corner to `width` x `height` px; one "Resize Shape" state. False (no state) as for `shape_set_active_fill`.
        fn shape_resize_active(view: Pin<&mut PictureView>, width: f64, height: f64) -> bool;

        /// Whether the layer at `path` is a shape layer.
        fn shape_is_shape(view: &PictureView, path: &QString) -> bool;

        /// Copy the shape layer at `path`'s fill and stroke into the shape-attributes clipboard. False (clipboard kept) when `path` is not a shape layer; no history.
        fn shape_copy_attributes(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Paste the shape-attributes clipboard onto the shape layer at `path`, replacing its fill and stroke; one "Paste Shape Attributes" state when it changed. False (no state) when `path` is not a shape layer or nothing changed.
        fn shape_paste_attributes(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Whether the shape-attributes clipboard holds a copy.
        fn shape_can_paste_attributes(view: &PictureView) -> bool;

        /// Rasterize the shape layer at `path` to ordinary pixels, dropping its shape/vector definition; one "Rasterize Shape" state. False (no state) when `path` is not a shape layer.
        fn shape_rasterize(view: Pin<&mut PictureView>, path: &QString) -> bool;
    }
}

/// The tool's kind and the outline `spec` draws.
fn outline(spec: &ShapeSpec) -> Option<(ShapeKind, Subpath)> {
    let kind = ShapeKind::from_i32(spec.kind)?;
    let options = ShapeOptions {
        kind,
        radii: [spec.r_tl, spec.r_tr, spec.r_br, spec.r_bl],
        sides: spec.sides.max(0) as u32,
        star: spec.star.then_some(spec.indent),
        smooth_corners: spec.smooth_corners,
        smooth_indents: spec.smooth_indents,
        weight: spec.weight,
        arrows: Arrowheads {
            start: spec.arrow_start,
            end: spec.arrow_end,
            width: spec.arrow_width,
            length: spec.arrow_length,
            concavity: spec.arrow_concavity,
        },
        custom: spec.custom.max(0) as usize,
    };
    let subpath = if spec.boxed {
        let (x, y) = (spec.x0.min(spec.x1), spec.y0.min(spec.y1));
        let rect = (x, y, (spec.x1 - spec.x0).abs(), (spec.y1 - spec.y0).abs());
        shape::outline_in_box(options, rect)?
    } else {
        shape::outline(
            options,
            (spec.x0, spec.y0),
            (spec.x1, spec.y1),
            spec.shift,
            spec.alt,
        )?
    };
    let subpath = if spec.align_edges {
        shape::align_edges(&subpath)
    } else {
        subpath
    };
    Some((kind, subpath))
}

fn stroke_of(color: u32, width: f64, align: i32) -> pictura_render::ShapeStroke {
    let c = rgba_from_argb(color);
    pictura_render::ShapeStroke {
        color: [c.r, c.g, c.b],
        width: width.round().clamp(1.0, 250.0) as u32,
        position: match align {
            0 => pictura_render::StrokePosition::Inside,
            2 => pictura_render::StrokePosition::Outside,
            _ => pictura_render::StrokePosition::Center,
        },
    }
}

/// The live origin of a Shape-mode `kind` drawn as `outline`: its box and
/// clamped radii. Polygons, Lines, and Custom Shapes were not live shapes in
/// CC 2015, so they have none.
fn live_shape(spec: &ShapeSpec, kind: ShapeKind, outline: &Subpath) -> Option<LiveShape> {
    let origin_type = match kind {
        ShapeKind::Rectangle => pictura_codec::ORIGIN_RECTANGLE,
        ShapeKind::RoundedRectangle => pictura_codec::ORIGIN_ROUNDED_RECTANGLE,
        ShapeKind::Ellipse => pictura_codec::ORIGIN_ELLIPSE,
        ShapeKind::Polygon | ShapeKind::Line | ShapeKind::CustomShape => return None,
    };
    let mut path = VectorPath::default();
    path.add_subpath(outline.clone());
    let bounds = path.subpath_bounds(0)?;
    let cap = (bounds.2 - bounds.0).min(bounds.3 - bounds.1) / 2.0;
    let radii = if kind == ShapeKind::RoundedRectangle {
        [spec.r_tl, spec.r_tr, spec.r_br, spec.r_bl].map(|r| r.clamp(0.0, cap))
    } else {
        [0.0; 4]
    };
    Some(LiveShape {
        origin_type,
        bounds,
        radii,
    })
}

fn label(kind: ShapeKind) -> String {
    format!("{} Tool", kind.layer_name())
}

fn shape_outline(spec: &ShapeSpec) -> Vec<f64> {
    let Some((_, subpath)) = outline(spec) else {
        return Vec::new();
    };
    subpath
        .points
        .iter()
        .flat_map(|p| {
            let (ix, iy) = p.in_handle.unwrap_or(p.anchor);
            let (ox, oy) = p.out_handle.unwrap_or(p.anchor);
            [p.anchor.0, p.anchor.1, ix, iy, ox, oy]
        })
        .collect()
}

fn shape_add_layer(mut view: Pin<&mut PictureView>, spec: &ShapeSpec, foreground: u32) -> QString {
    let Some((kind, subpath)) = outline(spec) else {
        return QString::default();
    };
    paint_timing::start("Shape layer");
    let t = std::time::Instant::now();
    let live = live_shape(spec, kind, &subpath);
    let c = rgba_from_argb(foreground);
    let created = {
        let mut rust = view.as_mut().rust_mut();
        let above = rust
            .layer_target
            .anchor()
            .map(str::to_string)
            .unwrap_or_default();
        match rust.session.doc.as_mut() {
            Some(doc) => pictura_render::add_shape_layer(
                doc,
                &above,
                [c.r, c.g, c.b, 255],
                kind.layer_name(),
                &subpath,
                live.as_ref(),
            ),
            None => String::new(),
        }
    };
    let mut reach = None;
    if let Some(doc) = view.as_mut().rust_mut().session.doc.as_mut() {
        let (width, height) = (doc.width, doc.height);
        if let Some(layer) = pictura_render::resolve_path_mut(doc, &created) {
            if spec.no_fill {
                pictura_render::set_shape_fill(layer, None);
            }
            let mut stroke_width = 0;
            if spec.stroke {
                let stroke = stroke_of(spec.stroke_color, spec.stroke_width, spec.stroke_align);
                stroke_width = stroke.width as i32;
                pictura_render::set_shape_stroke(layer, Some(&stroke));
            }
            reach = pictura_render::shape_bounds(layer, width, height)
                .map(|bounds| shape_reach(bounds, stroke_width));
        }
        // A clipped layer just above now clips to the new layer: what it shows
        // changes wherever it reaches, not only under the shape.
        if clipped_above(doc, &created) {
            reach = None;
        }
    }
    paint_timing::record("shape_add_layer", t.elapsed());
    if !created.is_empty() {
        view.as_mut().rust_mut().select_only_layer(&created);
        view.as_mut().clear_link_sets();
        // The new layer shows nowhere outside its outline and stroke, so only
        // that region of the composite changes.
        refresh_edit(view.as_mut(), reach, &created);
        view.as_mut().record(&label(kind));
    }
    paint_timing::report();
    QString::from(created.as_str())
}

/// The document pixels a shape with outline `bounds` (`left, top, right,
/// bottom`) and a stroke `stroke_width` wide can touch: the stroke's whole width
/// on every side (it may be outside), plus a pixel for the edge's coverage.
fn shape_reach(bounds: (f64, f64, f64, f64), stroke_width: i32) -> PsdRect {
    let (l, t, r, b) = bounds;
    let pad = stroke_width + 1;
    let at = |v: f64| v.clamp(i32::MIN as f64 / 2.0, i32::MAX as f64 / 2.0);
    PsdRect {
        top: at(t.floor()) as i32 - pad,
        left: at(l.floor()) as i32 - pad,
        bottom: at(b.ceil()) as i32 + pad,
        right: at(r.ceil()) as i32 + pad,
    }
}

/// Whether the sibling above the layer at `path` is clipped (or `path` does
/// not resolve, which is treated the same: refresh everything).
fn clipped_above(doc: &pictura_core::Document, path: &str) -> bool {
    let (parent, index) = match path.rsplit_once('/') {
        Some((parent, index)) => (Some(parent), index),
        None => (None, path),
    };
    let Ok(index) = index.parse::<usize>() else {
        return true;
    };
    let siblings = match parent {
        None => &doc.layers,
        Some(parent) => match pictura_render::resolve_path(doc, parent) {
            Some(layer) => &layer.children,
            None => return true,
        },
    };
    siblings.get(index + 1).is_some_and(|layer| layer.clipping)
}

fn shape_add_path(mut view: Pin<&mut PictureView>, spec: &ShapeSpec) -> bool {
    let Some((kind, subpath)) = outline(spec) else {
        return false;
    };
    super::paths::begin_drawing(view.as_mut());
    let added = super::paths::edit(view.as_mut(), |p| {
        p.add_subpath(subpath);
    })
    .is_some();
    if added {
        view.as_mut().edit(&label(kind), |_| Edit::Meta);
    }
    added
}

fn shape_fill_pixels(view: Pin<&mut PictureView>, spec: &ShapeSpec, foreground: u32) -> bool {
    let Some((kind, subpath)) = outline(spec) else {
        return false;
    };
    let paint = BucketPaint::Foreground(rgba_from_argb(foreground));
    paint_timing::start("Shape pixels");
    let applied = apply(view, &label(kind), |doc, path, rust| {
        let t = std::time::Instant::now();
        let mask = shape::coverage(&subpath, doc.width, doc.height);
        paint_timing::record("shape_coverage", t.elapsed());
        bucket::fill(
            doc,
            path,
            &mask,
            paint,
            PaintMode::Normal,
            1.0,
            selection(rust),
        )
    });
    paint_timing::report();
    applied
}

fn shape_custom_count() -> i32 {
    CUSTOM_SHAPE_NAMES.len() as i32
}

fn shape_custom_name(index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| CUSTOM_SHAPE_NAMES.get(i))
        .map_or_else(QString::default, |name| QString::from(*name))
}

fn shape_custom_preview(index: i32, size: i32) -> QImage {
    let (Ok(index), Ok(side)) = (usize::try_from(index), u32::try_from(size)) else {
        return QImage::default();
    };
    let coverage = shape::custom_shape_preview(index, side);
    if coverage.is_empty() {
        return QImage::default();
    }
    let rgba = coverage.iter().flat_map(|&a| [0, 0, 0, a]).collect();
    rgba_image(rgba, size, size)
}

/// Apply `edit` to the active shape layer; when it changed something,
/// recomposite and record `label`.
fn edit_active(
    mut view: Pin<&mut PictureView>,
    label: &str,
    edit: impl FnOnce(&mut Layer, u32, u32) -> bool,
) -> bool {
    let changed = {
        let mut rust = view.as_mut().rust_mut();
        let rust = &mut *rust;
        let (Some(doc), Some(path)) = (rust.session.doc.as_mut(), rust.layer_target.single())
        else {
            return false;
        };
        let (width, height) = (doc.width, doc.height);
        pictura_render::resolve_path_mut(doc, path)
            .filter(|layer| pictura_render::is_shape_layer(layer))
            .is_some_and(|layer| edit(layer, width, height))
    };
    if changed {
        view.as_mut().edit(label, |_| Edit::Full);
    }
    changed
}

fn argb(rgb: [u8; 3]) -> f64 {
    f64::from(0xff00_0000 | u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2]))
}

fn shape_active(view: &PictureView) -> Vec<f64> {
    let rust = view.rust();
    let Some((doc, layer)) = rust
        .session
        .doc
        .as_ref()
        .zip(rust.layer_target.single())
        .and_then(|(doc, path)| {
            pictura_render::resolve_path(doc, path)
                .filter(|l| pictura_render::is_shape_layer(l))
                .map(|l| (doc, l))
        })
    else {
        return Vec::new();
    };
    let Some((l, t, r, b)) = pictura_render::shape_bounds(layer, doc.width, doc.height) else {
        return Vec::new();
    };
    let fill = pictura_render::shape_fill(layer);
    let stroke = pictura_render::shape_stroke(layer);
    let flag = |on: bool| if on { 1.0 } else { 0.0 };
    // `vmsk` stores 8.24 fractions of the canvas, so a drawn size comes back a
    // few millionths off; the bar shows hundredths.
    let size = |v: f64| (v * 1000.0).round() / 1000.0;
    vec![
        size(r - l),
        size(b - t),
        flag(fill.is_some()),
        fill.map_or(0.0, argb),
        flag(stroke.is_some()),
        stroke.map_or(0.0, |s| argb(s.color)),
        stroke.map_or(0.0, |s| f64::from(s.width)),
        stroke.map_or(1.0, |s| match s.position {
            pictura_render::StrokePosition::Inside => 0.0,
            pictura_render::StrokePosition::Center => 1.0,
            pictura_render::StrokePosition::Outside => 2.0,
        }),
    ]
}

fn shape_set_active_fill(view: Pin<&mut PictureView>, on: bool, argb: u32) -> bool {
    let c = rgba_from_argb(argb);
    let fill = on.then_some([c.r, c.g, c.b]);
    edit_active(view, "Change Shape Fill", |layer, _, _| {
        pictura_render::set_shape_fill(layer, fill)
    })
}

fn shape_set_active_stroke(
    view: Pin<&mut PictureView>,
    on: bool,
    argb: u32,
    width: f64,
    align: i32,
) -> bool {
    let stroke = on.then(|| stroke_of(argb, width, align));
    edit_active(view, "Change Shape Stroke", |layer, _, _| {
        pictura_render::set_shape_stroke(layer, stroke.as_ref())
    })
}

fn shape_resize_active(view: Pin<&mut PictureView>, width: f64, height: f64) -> bool {
    edit_active(view, "Resize Shape", |layer, w, h| {
        pictura_render::resize_shape(layer, w, h, width, height)
    })
}

/// Whether the layer at `path` is a shape layer.
fn shape_is_shape(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::resolve_path(doc, &path.to_string()))
        .is_some_and(pictura_render::is_shape_layer)
}

/// A shape layer's fill-coverage thumbnail over the document extent at the
/// document's aspect ratio (`size` on the long edge); empty for a non-shape
/// layer, no fill colour, or a non-positive size.
pub(super) fn shape_thumbnail(
    layer: &pictura_core::Layer,
    doc: &pictura_core::Document,
    size: i32,
) -> QImage {
    let Some([r, g, b, _]) = pictura_render::shape_fill_color(layer) else {
        return QImage::default();
    };
    if size <= 0 || doc.width == 0 || doc.height == 0 {
        return QImage::default();
    }
    let (tw, th) = aspect_thumbnail_extent(doc.width, doc.height, size as u32);
    let scale_x = doc.width as f64 / tw as f64;
    let scale_y = doc.height as f64 / th as f64;
    let clip = pictura_core::PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    let mut rgba = vec![0u8; (tw * th * 4) as usize];
    for ty in 0..th {
        for tx in 0..tw {
            let x = ((tx as f64 + 0.5) * scale_x) as i32;
            let y = ((ty as f64 + 0.5) * scale_y) as i32;
            if x >= doc.width as i32 || y >= doc.height as i32 {
                continue;
            }
            let a = pictura_render::shape_coverage(layer, clip, x, y);
            let o = ((ty * tw + tx) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[r, g, b, a]);
        }
    }
    rgba_image(rgba, tw as i32, th as i32)
}

/// Copy the shape layer at `path`'s fill and stroke into the clipboard. False
/// (clipboard kept) for a non-shape layer; records no state.
fn shape_copy_attributes(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let attributes = {
        let rust = view.as_mut().rust_mut();
        rust.session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, &path))
            .and_then(pictura_render::copy_shape_attributes)
    };
    match attributes {
        Some(attributes) => {
            view.as_mut().rust_mut().shape_attributes = Some(attributes);
            true
        }
        None => false,
    }
}

/// Paste the clipboard onto the shape layer at `path`; one state when changed.
fn shape_paste_attributes(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let Some(attributes) = view.rust().shape_attributes else {
        return false;
    };
    let path = path.to_string();
    let changed = {
        let mut rust = view.as_mut().rust_mut();
        match rust.session.doc.as_mut() {
            Some(doc) => pictura_render::resolve_path_mut(doc, &path)
                .is_some_and(|layer| pictura_render::paste_shape_attributes(layer, &attributes)),
            None => false,
        }
    };
    if changed {
        view.as_mut().edit("Paste Shape Attributes", |_| Edit::Full);
    }
    changed
}

fn shape_can_paste_attributes(view: &PictureView) -> bool {
    view.rust().shape_attributes.is_some()
}

/// Rasterize the shape layer at `path`; one "Rasterize Shape" state when it
/// changed.
fn shape_rasterize(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let changed = {
        let mut rust = view.as_mut().rust_mut();
        match rust.session.doc.as_mut() {
            Some(doc) => pictura_render::rasterize_shape(doc, &path),
            None => false,
        }
    };
    if changed {
        view.as_mut().edit("Rasterize Shape", |_| Edit::Structural);
    }
    changed
}
