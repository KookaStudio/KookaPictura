use super::*;

use super::super::helpers::{paint_timing, rgba_from_argb};
use super::super::helpers_composite::{aspect_thumbnail_extent, rgba_image};
use super::super::paint_tools::fills::{apply, refresh_edit};
use crate::cxxqt_object::edit::{structural, Edit};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString};
use ffi::ShapeSpec;
use pictura_codec::LiveShape;
use pictura_core::path::{Subpath, VectorPath};
use pictura_core::shape::{self, Arrowheads, ShapeKind, ShapeOptions, CUSTOM_SHAPE_NAMES};
use pictura_core::{Layer, PsdRect};
use pictura_paint::bucket::{self, BucketPaint};
use pictura_paint::{Alpha, PaintMode};

/// The tool's kind and the outline `spec` draws.
pub(super) fn outline(spec: &ShapeSpec) -> Option<(ShapeKind, Subpath)> {
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
        proportion: (spec.proportion > 0.0).then_some(spec.proportion),
    };
    let snap =
        spec.snap_to_pixels && matches!(kind, ShapeKind::Rectangle | ShapeKind::RoundedRectangle);
    let subpath = if spec.boxed {
        let (mut x0, mut y0, mut x1, mut y1) = (spec.x0, spec.y0, spec.x1, spec.y1);
        if snap {
            x0 = x0.round();
            y0 = y0.round();
            x1 = x1.round();
            y1 = y1.round();
        }
        let (x, y) = (x0.min(x1), y0.min(y1));
        let rect = (x, y, (x1 - x0).abs(), (y1 - y0).abs());
        shape::outline_in_box(options, rect)?
    } else {
        let (x0, y0, x1, y1) = if snap {
            (
                spec.x0.round(),
                spec.y0.round(),
                spec.x1.round(),
                spec.y1.round(),
            )
        } else {
            (spec.x0, spec.y0, spec.x1, spec.y1)
        };
        shape::outline(options, (x0, y0), (x1, y1), spec.shift, spec.alt)?
    };
    let subpath = if spec.align_edges {
        shape::align_edges(&subpath)
    } else {
        subpath
    };
    Some((kind, subpath))
}

pub(super) fn stroke_of(color: u32, width: f64, align: i32) -> pictura_render::ShapeStroke {
    let c = rgba_from_argb(color);
    pictura_render::ShapeStroke::solid(
        [c.r, c.g, c.b],
        width.round().clamp(1.0, 250.0) as u32,
        match align {
            0 => pictura_render::StrokePosition::Inside,
            2 => pictura_render::StrokePosition::Outside,
            _ => pictura_render::StrokePosition::Center,
        },
    )
}

fn outlines(spec: &ShapeSpec) -> Option<(ShapeKind, Vec<Subpath>)> {
    if spec.kind != 5 || spec.custom_preset.is_empty() {
        return outline(spec).map(|(kind, path)| (kind, vec![path]));
    }
    use pictura_codec::preset_library::{decode, Format, Kind, Payload};
    let mut library = decode(Kind::CustomShapes, Format::Native, &spec.custom_preset)
        .ok()?
        .library;
    let Payload::Shape(preset) = library.entries.pop()?.payload else {
        return None;
    };
    let [top, left, bottom, right] = preset.bounds;
    let width = f64::from(right.checked_sub(left)?);
    let height = f64::from(bottom.checked_sub(top)?);
    let mut paths = preset.paths()?;
    let (x, y, w, h) = if spec.boxed {
        (
            spec.x0.min(spec.x1),
            spec.y0.min(spec.y1),
            (spec.x1 - spec.x0).abs(),
            (spec.y1 - spec.y0).abs(),
        )
    } else {
        shape::drag_rect(
            (spec.x0, spec.y0),
            (spec.x1, spec.y1),
            spec.shift.then_some(width / height),
            spec.alt,
        )
    };
    if w <= 0.0 || h <= 0.0 || paths.is_empty() {
        return None;
    }
    let map = |(u, v)| (x + u * w / width, y + v * h / height);
    for path in &mut paths {
        for point in &mut path.points {
            point.anchor = map(point.anchor);
            point.in_handle = point.in_handle.map(map);
            point.out_handle = point.out_handle.map(map);
        }
        if spec.align_edges {
            *path = shape::align_edges(path);
        }
    }
    Some((ShapeKind::CustomShape, paths))
}

fn text(bytes: &[u8]) -> Option<String> {
    std::str::from_utf8(bytes).ok().map(str::to_string)
}

fn gradient_from_payload(payload: &str) -> Option<pictura_render::GradientFillParams> {
    let mut parts = payload.split('|');
    let kind = match parts.next()? {
        "linear" => pictura_render::GradientKind::Linear,
        "radial" => pictura_render::GradientKind::Radial,
        "angled" => pictura_render::GradientKind::Angle,
        "reflected" => pictura_render::GradientKind::Reflected,
        "diamond" => pictura_render::GradientKind::Diamond,
        _ => return None,
    };
    let angle_deg: f32 = parts.next()?.parse().ok()?;
    let scale: f32 = parts.next()?.parse().ok()?;
    let reverse = parts.next()? == "1";
    let stops = parts
        .next()?
        .split(',')
        .filter(|stop| !stop.is_empty())
        .map(|stop| {
            let (location, hex) = stop.split_once(':')?;
            let location: u16 = location.parse().ok()?;
            let value = u32::from_str_radix(hex.trim_start_matches('#'), 16).ok()?;
            Some(pictura_render::GradientStop {
                location,
                color: [(value >> 16) as u8, (value >> 8) as u8, value as u8],
            })
        })
        .collect::<Option<Vec<_>>>()?;
    if stops.len() < 2 || !angle_deg.is_finite() || !scale.is_finite() {
        return None;
    }
    Some(pictura_render::GradientFillParams {
        stops,
        reverse,
        kind,
        angle_deg,
        scale,
    })
}

fn pattern_from_payload(payload: &str) -> Option<pictura_render::PatternFillParams> {
    let mut parts = payload.split('|');
    let pattern_id = parts.next()?.to_string();
    if pattern_id.is_empty() {
        return None;
    }
    let scale: f32 = parts.next()?.parse().ok()?;
    let link_with_layer = parts.next()? == "1";
    let x: i32 = parts.next()?.parse().ok()?;
    let y: i32 = parts.next()?.parse().ok()?;
    scale
        .is_finite()
        .then_some(pictura_render::PatternFillParams {
            pattern_id,
            scale,
            link_with_layer,
            origin: (x, y),
        })
}

fn gradient_payload(params: &pictura_render::GradientFillParams) -> String {
    let kind = match params.kind {
        pictura_render::GradientKind::Linear => "linear",
        pictura_render::GradientKind::Radial => "radial",
        pictura_render::GradientKind::Angle => "angled",
        pictura_render::GradientKind::Reflected => "reflected",
        pictura_render::GradientKind::Diamond => "diamond",
    };
    let stops = params
        .stops
        .iter()
        .map(|stop| {
            format!(
                "{}:{:02x}{:02x}{:02x}",
                stop.location, stop.color[0], stop.color[1], stop.color[2]
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{}|{}|{}|{}|{}",
        kind,
        params.angle_deg,
        params.scale,
        u8::from(params.reverse),
        stops
    )
}

fn pattern_payload(params: &pictura_render::PatternFillParams) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        params.pattern_id,
        params.scale,
        u8::from(params.link_with_layer),
        params.origin.0,
        params.origin.1
    )
}

fn fill_kind(spec: &ShapeSpec) -> i32 {
    if spec.fill_kind == 0 {
        if spec.no_fill {
            0
        } else {
            1
        }
    } else {
        spec.fill_kind
    }
}

fn fill_from_spec(spec: &ShapeSpec, foreground: u32) -> Option<pictura_render::ShapeFill> {
    match fill_kind(spec) {
        2 => gradient_from_payload(&text(&spec.fill_gradient)?)
            .map(pictura_render::ShapeFill::Gradient),
        3 => {
            pattern_from_payload(&text(&spec.fill_pattern)?).map(pictura_render::ShapeFill::Pattern)
        }
        0 => None,
        _ => {
            let c = rgba_from_argb(foreground);
            Some(pictura_render::ShapeFill::Solid([c.r, c.g, c.b]))
        }
    }
}

fn stroke_from_spec(spec: &ShapeSpec) -> Option<pictura_render::ShapeStroke> {
    if !spec.stroke {
        return None;
    }
    let kind = if spec.stroke_kind == 0 {
        1
    } else {
        spec.stroke_kind
    };
    let fill = match kind {
        2 => pictura_render::ShapeStrokeFill::Gradient {
            params: gradient_from_payload(&text(&spec.stroke_gradient)?)?,
            align_with_layer: true,
        },
        3 => pictura_render::ShapeStrokeFill::Pattern {
            params: pattern_from_payload(&text(&spec.stroke_pattern)?)?,
            angle_deg: 0.0,
        },
        _ => {
            let c = rgba_from_argb(spec.stroke_color);
            pictura_render::ShapeStrokeFill::Solid([c.r, c.g, c.b])
        }
    };
    Some(pictura_render::ShapeStroke {
        fill,
        width: spec.stroke_width.round().clamp(1.0, 250.0) as u32,
        position: match spec.stroke_align {
            0 => pictura_render::StrokePosition::Inside,
            2 => pictura_render::StrokePosition::Outside,
            _ => pictura_render::StrokePosition::Center,
        },
        line: match spec.stroke_line {
            1 => pictura_render::StrokeLine::Dashed,
            2 => pictura_render::StrokeLine::Dotted,
            _ => pictura_render::StrokeLine::Solid,
        },
    })
}

/// The live origin of a Shape-mode `kind` drawn as `outline`: its box and
/// clamped radii. Polygons, Lines, and Custom Shapes were not live shapes in
/// CC 2015, so they have none.
pub(super) fn live_shape(
    spec: &ShapeSpec,
    kind: ShapeKind,
    outline: &Subpath,
) -> Option<LiveShape> {
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

pub(super) fn label(kind: ShapeKind) -> String {
    format!("{} Tool", kind.layer_name())
}

pub(super) fn shape_outline(spec: &ShapeSpec) -> Vec<f64> {
    let Some((_, subpaths)) = outlines(spec) else {
        return Vec::new();
    };
    let mut coordinates = Vec::new();
    for subpath in subpaths {
        if !spec.custom_preset.is_empty() {
            coordinates.extend([
                f64::NAN,
                if subpath.closed { 1.0 } else { 0.0 },
                0.0,
                0.0,
                0.0,
                0.0,
            ]);
        }
        coordinates.extend(subpath.points.iter().flat_map(|p| {
            let (ix, iy) = p.in_handle.unwrap_or(p.anchor);
            let (ox, oy) = p.out_handle.unwrap_or(p.anchor);
            [p.anchor.0, p.anchor.1, ix, iy, ox, oy]
        }));
    }
    coordinates
}

pub(super) fn shape_add_layer(
    mut view: Pin<&mut PictureView>,
    spec: &ShapeSpec,
    foreground: u32,
) -> QString {
    let Some((kind, subpaths)) = outlines(spec) else {
        return QString::default();
    };
    paint_timing::start("Shape layer");
    let t = std::time::Instant::now();
    let live = live_shape(spec, kind, &subpaths[0]);
    let c = rgba_from_argb(foreground);
    let created = {
        let mut rust = view.as_mut().rust_mut();
        let above = rust
            .layer_target
            .anchor()
            .map(str::to_string)
            .unwrap_or_default();
        match rust.session.doc.as_mut() {
            Some(doc) => {
                let before = doc.layer_section_extra.clone();
                let fill_pattern = (fill_kind(spec) == 3)
                    .then(|| text(&spec.fill_pattern))
                    .flatten();
                let stroke_pattern = (spec.stroke && spec.stroke_kind == 3)
                    .then(|| text(&spec.stroke_pattern))
                    .flatten();
                if (fill_kind(spec) == 3 && fill_pattern.is_none())
                    || (spec.stroke && spec.stroke_kind == 3 && stroke_pattern.is_none())
                    || fill_pattern
                        .as_deref()
                        .is_some_and(|p| !ensure_shape_pattern(doc, p))
                    || stroke_pattern
                        .as_deref()
                        .is_some_and(|p| !ensure_shape_pattern(doc, p))
                {
                    doc.layer_section_extra = before;
                    return QString::default();
                }
                let created = pictura_render::add_shape_layer(
                    doc,
                    &above,
                    [c.r, c.g, c.b, 255],
                    kind.layer_name(),
                    &subpaths[0],
                    live.as_ref(),
                );
                if created.is_empty() {
                    doc.layer_section_extra = before;
                }
                created
            }
            None => String::new(),
        }
    };
    let mut reach = None;
    if let Some(doc) = view.as_mut().rust_mut().session.doc.as_mut() {
        let (width, height) = (doc.width, doc.height);
        if let Some(layer) = pictura_render::resolve_path_mut(doc, &created) {
            if subpaths.len() > 1 {
                pictura_render::set_layer_shape_paths(layer, &subpaths, width, height);
            }
            let fill = fill_from_spec(spec, foreground);
            pictura_render::set_shape_fill_content(layer, fill.as_ref());
            let mut stroke_width = 0;
            if let Some(stroke) = stroke_from_spec(spec) {
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

pub(super) fn shape_add_work_path_layer(
    mut view: Pin<&mut PictureView>,
    foreground: u32,
    label: &str,
) -> QString {
    let subpaths = {
        let rust = view.rust();
        let Some(doc) = rust.session.doc.as_ref() else {
            return QString::default();
        };
        doc.work_path.subpaths.clone()
    };
    if subpaths.iter().all(|subpath| subpath.points.len() < 2) {
        return QString::default();
    }
    paint_timing::start("Pen shape");
    let t = std::time::Instant::now();
    let c = rgba_from_argb(foreground);
    let created = {
        let mut rust = view.as_mut().rust_mut();
        let above = rust
            .layer_target
            .anchor()
            .map(str::to_string)
            .unwrap_or_default();
        let Some(doc) = rust.session.doc.as_mut() else {
            return QString::default();
        };
        pictura_render::add_shape_layer(
            doc,
            &above,
            [c.r, c.g, c.b, 255],
            "Shape",
            &subpaths[0],
            None,
        )
    };
    if created.is_empty() {
        paint_timing::report();
        return QString::default();
    }
    let mut reach = None;
    if let Some(doc) = view.as_mut().rust_mut().session.doc.as_mut() {
        let (width, height) = (doc.width, doc.height);
        if let Some(layer) = pictura_render::resolve_path_mut(doc, &created) {
            if subpaths.len() > 1 {
                pictura_render::set_layer_shape_paths(layer, &subpaths, width, height);
            }
            reach = pictura_render::shape_bounds(layer, width, height)
                .map(|bounds| shape_reach(bounds, 0));
        }
        doc.work_path = VectorPath::default();
        if clipped_above(doc, &created) {
            reach = None;
        }
    }
    view.as_mut().rust_mut().select_only_layer(&created);
    view.as_mut().clear_link_sets();
    refresh_edit(view.as_mut(), reach, &created);
    view.as_mut().record(label);
    paint_timing::record("shape_add_work_path_layer", t.elapsed());
    paint_timing::report();
    QString::from(created.as_str())
}

/// The document pixels a shape with outline `bounds` (`left, top, right,
/// bottom`) and a stroke `stroke_width` wide can touch: the stroke's whole width
/// on every side (it may be outside), plus a pixel for the edge's coverage.
pub(super) fn shape_reach(bounds: (f64, f64, f64, f64), stroke_width: i32) -> PsdRect {
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
pub(super) fn clipped_above(doc: &pictura_core::Document, path: &str) -> bool {
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
            Some(layer) => layer.children(),
            None => return true,
        },
    };
    siblings.get(index + 1).is_some_and(|layer| layer.clipping)
}

pub(super) fn shape_add_path(mut view: Pin<&mut PictureView>, spec: &ShapeSpec) -> bool {
    let Some((kind, subpaths)) = outlines(spec) else {
        return false;
    };
    super::super::paths::begin_drawing(view.as_mut());
    let added = super::super::paths::edit(view.as_mut(), |p| {
        for path in subpaths {
            p.add_subpath(path);
        }
    })
    .is_some();
    if added {
        view.as_mut().edit(&label(kind), |_| Edit::Meta);
    }
    added
}

pub(super) fn shape_fill_pixels(
    view: Pin<&mut PictureView>,
    spec: &ShapeSpec,
    foreground: u32,
) -> bool {
    let Some((kind, subpaths)) = outlines(spec) else {
        return false;
    };
    let paint = BucketPaint::Foreground(rgba_from_argb(foreground));
    let mode = match spec.fill_mode {
        1 => PaintMode::Dissolve,
        2 => PaintMode::Behind,
        3 => PaintMode::Clear,
        4 => PaintMode::Multiply,
        _ => PaintMode::Normal,
    };
    let opacity = (spec.fill_opacity / 100.0).clamp(0.0, 1.0) as f32;
    let harden = |mask: &mut [u8]| {
        if !spec.fill_antialias {
            for value in mask {
                *value = if *value >= 128 { 255 } else { 0 };
            }
        }
    };
    paint_timing::start("Shape pixels");
    let applied = apply(view, &label(kind), |doc, path, t, selection| {
        let t0 = std::time::Instant::now();
        if !t.is_mask {
            // A layer target is the document: rasterize and fill the shape's
            // own box, not a document-sized mask (#1185).
            let (bounds, mut mask) =
                shape::paths_coverage_box(&subpaths, t.document_size.0, t.document_size.1)?;
            harden(&mut mask);
            paint_timing::record("shape_coverage", t0.elapsed());
            return bucket::fill_within(
                doc,
                path,
                bounds,
                Some(&mask),
                paint,
                mode,
                opacity,
                selection,
                Alpha::Blend,
            );
        }
        let mut mask = shape::paths_coverage(&subpaths, t.document_size.0, t.document_size.1);
        harden(&mut mask);
        paint_timing::record("shape_coverage", t0.elapsed());
        bucket::fill(
            doc,
            path,
            &t.coverage(&mask),
            paint,
            mode,
            opacity,
            selection,
            Alpha::Blend,
        )
    });
    paint_timing::report();
    applied
}

pub(super) fn shape_custom_count() -> i32 {
    CUSTOM_SHAPE_NAMES.len() as i32
}

pub(super) fn shape_custom_name(index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| CUSTOM_SHAPE_NAMES.get(i))
        .map_or_else(QString::default, |name| QString::from(*name))
}

pub(super) fn shape_custom_preview(index: i32, size: i32) -> QImage {
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

fn ensure_shape_pattern(doc: &mut pictura_core::Document, payload: &str) -> bool {
    let Some(params) = pattern_from_payload(payload) else {
        return false;
    };
    // Legacy references can be unresolved and still use the renderer's fallback.
    let has_snapshot = payload.split('|').nth(5).is_some();
    super::super::impl_layers::ensure_payload_pattern(doc, &params.pattern_id, payload)
        || !has_snapshot
}

/// Apply `edit` to the active shape layer; when it changed something,
/// recomposite and record `label`.
pub(super) fn edit_active(
    mut view: Pin<&mut PictureView>,
    label: &str,
    edit: impl FnOnce(&mut Layer, u32, u32) -> bool,
) -> bool {
    edit_active_pattern(view.as_mut(), label, None, edit)
}

fn edit_active_pattern(
    mut view: Pin<&mut PictureView>,
    label: &str,
    pattern: Option<&str>,
    edit: impl FnOnce(&mut Layer, u32, u32) -> bool,
) -> bool {
    let edit = view.as_mut().edit(label, |rust| {
        let (Some(doc), Some(path)) = (rust.session.doc.as_mut(), rust.layer_target.single())
        else {
            return Edit::None;
        };
        let path = path.to_string();
        let patterns_before = pattern.map(|_| doc.layer_section_extra.clone());
        if pattern.is_some_and(|payload| !ensure_shape_pattern(doc, payload)) {
            if let Some(saved) = patterns_before {
                doc.layer_section_extra = saved;
            }
            return Edit::None;
        }
        let before = pictura_render::damage::influence(doc, &path);
        let (width, height) = (doc.width, doc.height);
        let changed = pictura_render::resolve_path_mut(doc, path.as_str())
            .filter(|layer| pictura_render::is_shape_layer(layer))
            .is_some_and(|layer| edit(layer, width, height));
        if changed {
            let after = pictura_render::damage::influence(doc, &path);
            match pictura_render::damage::union(before, after) {
                Some(rect) => Edit::regions(vec![rect], true),
                None => Edit::Full,
            }
        } else {
            if let Some(saved) = patterns_before {
                doc.layer_section_extra = saved;
            }
            Edit::None
        }
    });
    !matches!(edit, Edit::None)
}

pub(super) fn argb(rgb: [u8; 3]) -> f64 {
    f64::from(0xff00_0000 | u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2]))
}

pub(super) fn shape_active(view: &PictureView) -> Vec<f64> {
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
        stroke.as_ref().map_or(0.0, |s| match s.fill {
            pictura_render::ShapeStrokeFill::Solid(color) => argb(color),
            _ => 0.0,
        }),
        stroke.as_ref().map_or(0.0, |s| f64::from(s.width)),
        stroke.as_ref().map_or(1.0, |s| match s.position {
            pictura_render::StrokePosition::Inside => 0.0,
            pictura_render::StrokePosition::Center => 1.0,
            pictura_render::StrokePosition::Outside => 2.0,
        }),
    ]
}

fn active_shape(view: &PictureView) -> Option<(&pictura_core::Document, &Layer)> {
    let rust = view.rust();
    let (doc, path) = rust.session.doc.as_ref().zip(rust.layer_target.single())?;
    let layer = pictura_render::resolve_path(doc, path)
        .filter(|layer| pictura_render::is_shape_layer(layer))?;
    Some((doc, layer))
}

fn style_json(
    fill: Option<pictura_render::ShapeFill>,
    stroke: Option<pictura_render::ShapeStroke>,
) -> String {
    let fill = match fill {
        None => serde_json::json!({"kind": 0, "color": 0, "gradient": "", "pattern": ""}),
        Some(pictura_render::ShapeFill::Solid(color)) => {
            serde_json::json!({"kind": 1, "color": argb(color), "gradient": "", "pattern": ""})
        }
        Some(pictura_render::ShapeFill::Gradient(params)) => serde_json::json!({
            "kind": 2,
            "color": 0,
            "gradient": gradient_payload(&params),
            "pattern": "",
        }),
        Some(pictura_render::ShapeFill::Pattern(params)) => serde_json::json!({
            "kind": 3,
            "color": 0,
            "gradient": "",
            "pattern": pattern_payload(&params),
        }),
    };
    let stroke = match stroke {
        None => serde_json::json!({
            "kind": 0, "color": 0, "gradient": "", "pattern": "",
            "width": 3, "align": 1, "line": 0,
        }),
        Some(stroke) => {
            let (kind, color, gradient, pattern) = match &stroke.fill {
                pictura_render::ShapeStrokeFill::Solid(color) => {
                    (1, argb(*color), String::new(), String::new())
                }
                pictura_render::ShapeStrokeFill::Gradient { params, .. } => {
                    (2, 0.0, gradient_payload(params), String::new())
                }
                pictura_render::ShapeStrokeFill::Pattern { params, .. } => {
                    (3, 0.0, String::new(), pattern_payload(params))
                }
            };
            serde_json::json!({
                "kind": kind,
                "color": color,
                "gradient": gradient,
                "pattern": pattern,
                "width": stroke.width,
                "align": match stroke.position {
                    pictura_render::StrokePosition::Inside => 0,
                    pictura_render::StrokePosition::Center => 1,
                    pictura_render::StrokePosition::Outside => 2,
                },
                "line": match stroke.line {
                    pictura_render::StrokeLine::Solid => 0,
                    pictura_render::StrokeLine::Dashed => 1,
                    pictura_render::StrokeLine::Dotted => 2,
                },
            })
        }
    };
    serde_json::json!({"fill": fill, "stroke": stroke}).to_string()
}

pub(super) fn shape_active_style(view: &PictureView) -> QString {
    let Some((_, layer)) = active_shape(view) else {
        return QString::default();
    };
    QString::from(
        style_json(
            pictura_render::shape_fill_content(layer),
            pictura_render::shape_stroke(layer),
        )
        .as_str(),
    )
}

pub(super) fn shape_set_active_fill_style(
    view: Pin<&mut PictureView>,
    kind: i32,
    argb: u32,
    gradient: &str,
    pattern: &str,
) -> bool {
    let fill = match kind {
        0 => None,
        2 => gradient_from_payload(gradient).map(pictura_render::ShapeFill::Gradient),
        3 => pattern_from_payload(pattern).map(pictura_render::ShapeFill::Pattern),
        _ => {
            let c = rgba_from_argb(argb);
            Some(pictura_render::ShapeFill::Solid([c.r, c.g, c.b]))
        }
    };
    if kind == 3 && fill.is_none() {
        return false;
    }
    edit_active_pattern(
        view,
        "Change Shape Fill",
        (kind == 3).then_some(pattern),
        |layer, _, _| pictura_render::set_shape_fill_content(layer, fill.as_ref()),
    )
}

pub(super) fn shape_set_active_stroke_style(
    view: Pin<&mut PictureView>,
    kind: i32,
    argb: u32,
    gradient: &str,
    pattern: &str,
    width: f64,
    align: i32,
    line: i32,
) -> bool {
    let stroke = match kind {
        0 => None,
        _ => {
            let fill = match kind {
                2 => gradient_from_payload(gradient).map(|params| {
                    pictura_render::ShapeStrokeFill::Gradient {
                        params,
                        align_with_layer: true,
                    }
                }),
                3 => pattern_from_payload(pattern).map(|params| {
                    pictura_render::ShapeStrokeFill::Pattern {
                        params,
                        angle_deg: 0.0,
                    }
                }),
                _ => {
                    let c = rgba_from_argb(argb);
                    Some(pictura_render::ShapeStrokeFill::Solid([c.r, c.g, c.b]))
                }
            };
            let Some(fill) = fill else {
                return false;
            };
            Some(pictura_render::ShapeStroke {
                fill,
                width: width.round().clamp(1.0, 250.0) as u32,
                position: match align {
                    0 => pictura_render::StrokePosition::Inside,
                    2 => pictura_render::StrokePosition::Outside,
                    _ => pictura_render::StrokePosition::Center,
                },
                line: match line {
                    1 => pictura_render::StrokeLine::Dashed,
                    2 => pictura_render::StrokeLine::Dotted,
                    _ => pictura_render::StrokeLine::Solid,
                },
            })
        }
    };
    edit_active_pattern(
        view,
        "Change Shape Stroke",
        (kind == 3).then_some(pattern),
        |layer, _, _| pictura_render::set_shape_stroke(layer, stroke.as_ref()),
    )
}

pub(super) fn shape_active_outline(view: &PictureView) -> Vec<f64> {
    let Some((doc, layer)) = active_shape(view) else {
        return Vec::new();
    };
    pictura_render::layer_shape_paths(layer, doc.width, doc.height)
        .map(|paths| super::super::shape_coords::encode(&paths))
        .unwrap_or_default()
}

pub(super) fn shape_replace_active_outline(view: Pin<&mut PictureView>, coords: &[f64]) -> bool {
    let Some(paths) = super::super::shape_coords::decode(coords) else {
        return false;
    };
    edit_active(view, "Path Operation", |layer, width, height| {
        if pictura_render::layer_shape_paths(layer, width, height).as_deref()
            == Some(paths.as_slice())
        {
            return false;
        }
        pictura_render::set_layer_shape_paths(layer, &paths, width, height);
        pictura_render::set_layer_live_shape(layer, None);
        true
    })
}

pub(super) fn shape_set_active_fill(view: Pin<&mut PictureView>, on: bool, argb: u32) -> bool {
    let c = rgba_from_argb(argb);
    let fill = on.then_some([c.r, c.g, c.b]);
    edit_active(view, "Change Shape Fill", |layer, _, _| {
        pictura_render::set_shape_fill(layer, fill)
    })
}

pub(super) fn shape_set_active_stroke(
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

pub(super) fn shape_resize_active(view: Pin<&mut PictureView>, width: f64, height: f64) -> bool {
    edit_active(view, "Resize Shape", |layer, w, h| {
        pictura_render::resize_shape(layer, w, h, width, height)
    })
}

/// Whether the layer at `path` is a shape layer.
pub(super) fn shape_is_shape(view: &PictureView, path: &QString) -> bool {
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
pub(in crate::cxxqt_object) fn shape_thumbnail(
    layer: &pictura_core::Layer,
    doc: &pictura_core::Document,
    size: i32,
) -> QImage {
    let Some(fill) = pictura_render::shape_fill_content(layer) else {
        return QImage::default();
    };
    if size <= 0 || doc.width == 0 || doc.height == 0 {
        return QImage::default();
    }
    let (tw, th) = aspect_thumbnail_extent(doc.width, doc.height, size as u32);
    let source: Vec<[u8; 4]> = match &fill {
        pictura_render::ShapeFill::Solid([r, g, b]) => {
            vec![[*r, *g, *b, 255]; (tw * th) as usize]
        }
        pictura_render::ShapeFill::Gradient(params) => {
            pictura_render::gradient_rgba(tw as i32, th as i32, params)
        }
        pictura_render::ShapeFill::Pattern(params) => {
            let patterns = pictura_codec::decode_patterns(doc);
            pictura_render::pattern_preview_rgba(&patterns, params, layer.rect, tw, th)
        }
    };
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
            let color = source
                .get((ty * tw + tx) as usize)
                .copied()
                .unwrap_or([0, 0, 0, 255]);
            let o = ((ty * tw + tx) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[
                color[0],
                color[1],
                color[2],
                (u32::from(color[3]) * u32::from(a) / 255) as u8,
            ]);
        }
    }
    rgba_image(rgba, tw as i32, th as i32)
}

/// Copy the shape layer at `path`'s fill and stroke into the clipboard. False
/// (clipboard kept) for a non-shape layer; records no state.
pub(super) fn shape_copy_attributes(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
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
pub(super) fn shape_paste_attributes(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let Some(attributes) = view.rust().shape_attributes.clone() else {
        return false;
    };
    let path = path.to_string();
    let mut changed = false;
    view.as_mut().edit("Paste Shape Attributes", |rust| {
        let before = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &path));
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        changed = pictura_render::resolve_path_mut(doc, &path)
            .is_some_and(|layer| pictura_render::paste_shape_attributes(layer, &attributes));
        if changed {
            let after = pictura_render::damage::influence(doc, &path);
            match pictura_render::damage::union(before, after) {
                Some(rect) => Edit::regions(vec![rect], true),
                None => Edit::Full,
            }
        } else {
            Edit::None
        }
    });
    changed
}

pub(super) fn shape_can_paste_attributes(view: &PictureView) -> bool {
    view.rust().shape_attributes.is_some()
}

/// Rasterize the shape layer at `path`; one "Rasterize Shape" state when it
/// changed.
pub(super) fn shape_rasterize(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let mut changed = false;
    view.as_mut().edit("Rasterize Shape", |rust| {
        let before = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &path));
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        changed = pictura_render::rasterize_shape(doc, &path);
        if changed {
            let after = pictura_render::damage::influence(doc, &path);
            structural(before, after)
        } else {
            Edit::None
        }
    });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::shape::outline_in_box;
    use pictura_core::{BitDepth, ColorMode, Document};

    #[test]
    fn extreme_document_shape_thumbnail_aspect_is_clamped() {
        for (width, height, expected) in [(200u32, 20u32, (24, 8)), (20, 200, (8, 24))] {
            let mut doc = Document::new(width, height, ColorMode::Rgb, BitDepth::Eight);
            let outline = outline_in_box(
                ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
                (4.0, 4.0, width as f64 - 8.0, height as f64 - 8.0),
            )
            .expect("outline");
            let path = pictura_render::add_shape_layer(
                &mut doc,
                "",
                [0, 0, 0, 255],
                "Shape",
                &outline,
                None,
            );
            let layer = pictura_render::resolve_path(&doc, &path).expect("shape layer");
            let image = shape_thumbnail(layer, &doc, 24);
            assert_eq!((image.width(), image.height()), expected);
        }
    }
}
