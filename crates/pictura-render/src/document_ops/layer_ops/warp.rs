//! Edit ▸ Transform ▸ Warp: a layer resampled through a [`Warp`] (a preset
//! style or a custom bicubic Bezier mesh, `pictura_core::warp`).
//!
//! The warp is a forward map in document space over the layer's rect, followed
//! by the session box's projective map (a Distort or Perspective made before
//! Warp stays exact, as Photoshop keeps `Trnf` apart from `warp`); pixels, the
//! layer mask and a retained 16/32-bit store all go through the same
//! triangulation (`warp_raster`), so they stay in register. Each is rasterized
//! a strip at a time into tiled planes the evictor can spill (#1292). The mask's outside
//! reads as its default colour, as photocraft's `warp_mesh_gray` has it.
//! Ported from photocraft's `engine::warp_cmds` (see `NOTICE.md`); the preset
//! shapes are measured approximations, not verified parity.
//!
//! The op reuses [`prepare_layer`] / [`write_layer`] from `transform.rs`, so its
//! refusal, channel-less materialization, mask, and smart-object rules match the
//! similarity and projective ops.

use pictura_core::warp::{Warp, WarpStyle};
use pictura_core::{Channel, Document, LayerMask, Plane, PsdRect, Sample, Samples, SourceChannels};

use super::kernel::Interpolation;
use super::shape_layer::map_shape_layer;
use super::transform::{
    prepare_layer, shape_rect, solve_homography, write_layer, LayerOutput, PlaneMap,
};
use super::warp_raster::{triangulate, warp_tiled, Kind, Source, SrcPlane, Triangles};

fn rect_of(d: (i32, i32, i32, i32)) -> PsdRect {
    PsdRect {
        left: d.0,
        top: d.1,
        right: d.2,
        bottom: d.3,
    }
}

/// Whether `warp` can be evaluated: finite parameters and, for a custom warp, a
/// well-formed finite mesh.
fn warp_is_valid(warp: &Warp) -> bool {
    let finite = [warp.bend, warp.h_distort, warp.v_distort]
        .iter()
        .chain(&warp.bounds)
        .all(|v| v.is_finite());
    let mesh_ok = match (&warp.mesh, warp.style) {
        (Some(m), WarpStyle::Custom) => {
            m.is_valid()
                && m.points
                    .iter()
                    .all(|p| p[0].is_finite() && p[1].is_finite())
        }
        _ => true,
    };
    finite && mesh_ok
}

/// The corners of `bounds` in TL, TR, BR, BL order.
fn bounds_corners(b: [f64; 4]) -> [(f64, f64); 4] {
    [(b[0], b[1]), (b[2], b[1]), (b[2], b[3]), (b[0], b[3])]
}

/// `warp.map` extended past the box for a mask wider than the layer: a point
/// outside moves with the nearest box point, so a translating warp moves the
/// whole mask.
fn map_beyond_box(warp: &Warp, x: f64, y: f64) -> (f64, f64) {
    let b = warp.bounds;
    let (cx, cy) = (x.clamp(b[0], b[2]), y.clamp(b[1], b[3]));
    let (mx, my) = warp.map(cx, cy);
    (mx + x - cx, my + y - cy)
}

/// An 8-bit output plane of [`warp_tiled`].
fn into_u8(samples: Samples) -> Plane<u8> {
    match samples {
        Samples::U8(p) => p,
        _ => unreachable!("warped as Kind::U8"),
    }
}

/// Warp a gray plane over `rect` whose outside reads as `default` (a layer
/// mask) into a plane of `kind`: the content is warped with full coverage and
/// flattened back onto the default.
pub(super) fn warp_gray(
    plane: SrcPlane<'_>,
    kind: Kind,
    rect: PsdRect,
    default: f32,
    t: &Triangles,
    interp: Interpolation,
) -> Samples {
    let src = Source {
        planes: vec![plane, SrcPlane::Ones],
        rect,
        alpha: Some(1),
    };
    let flatten = |planes: &mut Vec<Vec<f32>>| {
        let alpha = planes.pop().unwrap_or_default();
        if let Some(gray) = planes.last_mut() {
            gray.iter_mut()
                .zip(&alpha)
                .for_each(|(g, a)| *g = *g * a + default * (1.0 - a));
        }
    };
    warp_tiled(&src, t, interp, false, &[kind], &flatten)
        .pop()
        .expect("one output plane")
}

/// The warped layer mask, its triangulation (`None` when the mask has no area
/// or a plane disagrees with its rect), and its new rect.
fn warp_mask(
    mask: &LayerMask,
    f: &(dyn Fn(f64, f64) -> (f64, f64) + Sync),
    interp: Interpolation,
) -> Option<(LayerMask, Triangles)> {
    let rect = mask.rect;
    let t = triangulate(rect, f)?;
    let (dl, dt, dr, db) = t.dest;
    let interp = interp.resolve(
        (rect.width() as u64) * (rect.height() as u64),
        ((dr - dl) as u64) * ((db - dt) as u64),
    );
    let data = match mask.data.as_ref() {
        Some(data) => {
            if data.len() != (rect.width() as usize) * (rect.height() as usize) {
                return None;
            }
            let default = mask.default_color.to_unit() as f32;
            let gray = warp_gray(SrcPlane::U8(data), Kind::U8, rect, default, &t, interp);
            Some(into_u8(gray))
        }
        None => None,
    };
    let warped = LayerMask {
        rect: rect_of(t.dest),
        data,
        ..mask.clone()
    };
    Some((warped, t))
}

/// Warp a retained native store: the layer planes through `t` (alpha `-1`
/// premultiplying the rest), the `-2` mask plane through `mask` onto `default`.
fn warp_store(
    store: &SourceChannels,
    rect: PsdRect,
    t: &Triangles,
    mask: Option<(&LayerMask, &Triangles)>,
    interp: Interpolation,
) -> SourceChannels {
    let layer: Vec<&(i16, Samples)> = store.planes.iter().filter(|(id, _)| *id != -2).collect();
    let hdr = layer.iter().any(|(_, s)| matches!(s, Samples::F32(_)));
    let src = Source {
        planes: layer.iter().map(|(_, s)| SrcPlane::of(s)).collect(),
        rect,
        alpha: layer.iter().position(|(id, _)| *id == -1),
    };
    let kinds: Vec<Kind> = layer.iter().map(|(_, s)| Kind::of(s)).collect();
    let warped = warp_tiled(&src, t, interp, hdr, &kinds, &|_| {});
    let mut planes: Vec<(i16, Samples)> = layer
        .iter()
        .zip(warped)
        .map(|((id, _), plane)| (*id, plane))
        .collect();
    if let Some((id, like)) = store.planes.iter().find(|(id, _)| *id == -2) {
        let warped = match mask {
            Some((m, mt)) if like.len() == (m.rect.width() * m.rect.height()) as usize => {
                let default = m.default_color.to_unit() as f32;
                let plane = SrcPlane::of(like);
                warp_gray(plane, Kind::of(like), m.rect, default, mt, interp)
            }
            _ => like.clone(),
        };
        planes.push((*id, warped));
    }
    planes.sort_by_key(|(id, _)| *id);
    SourceChannels::new(store.depth, rect_of(t.dest), planes)
}

/// Warp the layer at `path` through `warp` (document space), then send the
/// warp box's corners onto `quad` (TL, TR, BR, BL) projectively; `None` keeps
/// the box where it is. Resamples with `interpolation`.
///
/// Refuses (leaving `doc` bit-identical) for every case `transform_layer`
/// refuses, for an identity or malformed warp with no box change, a singular
/// `quad`, a mask that cannot follow the pixels, and when the warped box is
/// degenerate, out of range, or too large. A shape layer warps its outline's
/// points instead of resampling (`map_shape_layer`). `pure_translate` is always false: a
/// warp drops an unmodeled raw channel stream.
pub fn transform_layer_warp(
    doc: &mut Document,
    path: &str,
    warp: &Warp,
    quad: Option<[(f64, f64); 4]>,
    interpolation: Interpolation,
) -> bool {
    let corners = bounds_corners(warp.bounds);
    let quad = quad.filter(|q| {
        q.iter()
            .zip(&corners)
            .any(|(a, b)| (a.0 - b.0).abs() > 1e-9 || (a.1 - b.1).abs() > 1e-9)
    });
    if !warp_is_valid(warp) || (warp.is_identity() && quad.is_none()) {
        return false;
    }
    let frame = match quad {
        Some(q) => match solve_homography(corners, q) {
            Some(h) => Some(h),
            None => return false,
        },
        None => None,
    };
    let project = |(x, y): (f64, f64)| match &frame {
        Some(h) => h.forward(x, y).unwrap_or((f64::NAN, f64::NAN)),
        None => (x, y),
    };
    if shape_rect(doc, path).is_some() {
        return map_shape_layer(doc, path, None, |x, y| {
            Some(project(map_beyond_box(warp, x, y)))
        });
    }
    let Some(prepared) = prepare_layer(doc, path) else {
        return false;
    };
    let f = |x: f64, y: f64| project(warp.map(x, y));
    let Some(t) = triangulate(prepared.rect, &f) else {
        return false;
    };
    let (sw, sh) = (prepared.w as usize, prepared.h as usize);
    let (dl, dt, dr, db) = t.dest;
    if let Some(job) = pictura_core::job::current() {
        // One pass of `warp_tiled` per layer, mask, and retained store.
        let passes = 1
            + u64::from(prepared.mask.is_some())
            + u64::from(prepared.native.is_some()) * (1 + u64::from(prepared.mask.is_some()));
        job.begin_stage((db - dt).max(0) as u64 * passes);
    }
    let interp = interpolation.resolve((sw * sh) as u64, ((dr - dl) as u64) * ((db - dt) as u64));
    let src = Source {
        planes: prepared
            .source_channels
            .iter()
            .map(|c| SrcPlane::U8(&c.data))
            .collect(),
        rect: prepared.rect,
        alpha: prepared.source_channels.iter().position(|c| c.id == -1),
    };
    let kinds = vec![Kind::U8; prepared.source_channels.len()];
    let warped = warp_tiled(&src, &t, interp, false, &kinds, &|_| {});
    let channels: Vec<Channel> = prepared
        .source_channels
        .iter()
        .zip(warped)
        .map(|(c, plane)| Channel {
            id: c.id,
            data: into_u8(plane),
        })
        .collect();
    let fm = |x: f64, y: f64| project(map_beyond_box(warp, x, y));
    // A mask with area that cannot be warped would stay behind while the
    // pixels move, hiding them: refuse instead.
    let mask = match prepared.mask.as_ref() {
        Some(m) if m.rect.width() > 0 && m.rect.height() > 0 => {
            match warp_mask(m, &fm, interpolation) {
                Some(warped) => Some(warped),
                None => return false,
            }
        }
        _ => None,
    };
    let native = prepared.native.as_ref().map(|store| {
        let mask_info = prepared
            .mask
            .as_ref()
            .zip(mask.as_ref())
            .map(|(m, (_, mt))| (m, mt));
        warp_store(store, prepared.rect, &t, mask_info, interp)
    });
    if super::transform::job_cancelled() {
        return false;
    }
    let out = LayerOutput {
        channels,
        mask: mask.map(|(m, _)| m),
        native,
        dest: t.dest,
    };
    write_layer(doc, path, &prepared, out, false)
}
