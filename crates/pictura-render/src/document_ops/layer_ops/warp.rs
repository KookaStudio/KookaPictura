//! Edit ▸ Transform ▸ Warp: a layer resampled through a [`Warp`] (a preset
//! style or a custom bicubic Bezier mesh, `pictura_core::warp`).
//!
//! The warp is a forward map in document space over the layer's rect, followed
//! by the session box's projective map (a Distort or Perspective made before
//! Warp stays exact, as Photoshop keeps `Trnf` apart from `warp`); pixels, the
//! layer mask and a retained 16/32-bit store all go through the same
//! triangulation (`warp_raster`), so they stay in register. The mask's outside
//! reads as its default colour, as photocraft's `warp_mesh_gray` has it.
//! Ported from photocraft's `engine::warp_cmds` (see `NOTICE.md`); the preset
//! shapes are measured approximations, not verified parity.
//!
//! The op reuses [`prepare_layer`] / [`write_layer`] from `transform.rs`, so its
//! refusal, channel-less materialization, mask, and smart-object rules match the
//! similarity and projective ops.

use pictura_core::warp::{Warp, WarpStyle};
use pictura_core::{Channel, Document, LayerMask, PsdRect, Sample, Samples, SourceChannels};

use super::kernel::Interpolation;
use super::shape_layer::map_shape_layer;
use super::transform::{
    prepare_layer, shape_rect, solve_homography, write_layer, LayerOutput, PlaneMap,
};
use super::warp_raster::{triangulate, warp_planes, Planes, Triangles};

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

/// Planes of one sample type to unit `f32`: 8/16-bit through [`Sample`], 32-bit
/// as stored (unclamped, NaN to zero).
fn unit_planes(planes: &[&Samples]) -> Vec<Vec<f32>> {
    planes
        .iter()
        .map(|s| match s {
            Samples::U8(v) => v.iter().map(|x| x.to_unit() as f32).collect(),
            Samples::U16(v) => v.iter().map(|x| x.to_unit() as f32).collect(),
            Samples::F32(v) => v
                .iter()
                .map(|x| if x.is_nan() { 0.0 } else { *x })
                .collect(),
        })
        .collect()
}

/// Unit `f32` back to the sample type of `like`.
fn to_samples(like: &Samples, plane: Vec<f32>) -> Samples {
    match like {
        Samples::U8(_) => Samples::U8(
            plane
                .into_iter()
                .map(|v| u8::from_unit(f64::from(v)))
                .collect(),
        ),
        Samples::U16(_) => Samples::U16(
            plane
                .into_iter()
                .map(|v| u16::from_unit(f64::from(v)))
                .collect(),
        ),
        Samples::F32(_) => Samples::F32(plane.into()),
    }
}

/// Warp a gray plane over `rect` whose outside reads as `default` (a layer
/// mask): the content is warped with full coverage and flattened back onto the
/// default.
fn warp_gray(
    plane: Vec<f32>,
    rect: PsdRect,
    default: f32,
    t: &Triangles,
    interp: Interpolation,
) -> Vec<f32> {
    let (w, h) = (rect.width() as usize, rect.height() as usize);
    let ones = vec![1.0f32; plane.len()];
    let src = Planes::new(vec![plane, ones], w, h, Some(1));
    let mut out = warp_planes(&src, rect, t, interp, false);
    let alpha = out.pop().unwrap_or_default();
    let mut gray = out.pop().unwrap_or_default();
    gray.iter_mut()
        .zip(&alpha)
        .for_each(|(g, a)| *g = *g * a + default * (1.0 - a));
    gray
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
            let plane = data.iter().map(|v| v.to_unit() as f32).collect();
            let default = mask.default_color.to_unit() as f32;
            let gray = warp_gray(plane, rect, default, &t, interp);
            Some(
                gray.into_iter()
                    .map(|v| u8::from_unit(f64::from(v)))
                    .collect::<Vec<u8>>()
                    .into(),
            )
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
    let (w, h) = (rect.width() as usize, rect.height() as usize);
    let layer: Vec<&(i16, Samples)> = store.planes.iter().filter(|(id, _)| *id != -2).collect();
    let hdr = layer.iter().any(|(_, s)| matches!(s, Samples::F32(_)));
    let alpha = layer.iter().position(|(id, _)| *id == -1);
    let samples: Vec<&Samples> = layer.iter().map(|(_, s)| s).collect();
    let src = Planes::new(unit_planes(&samples), w, h, alpha);
    let warped = warp_planes(&src, rect, t, interp, hdr);
    let mut planes: Vec<(i16, Samples)> = layer
        .iter()
        .zip(warped)
        .map(|((id, like), plane)| (*id, to_samples(like, plane)))
        .collect();
    if let Some((id, like)) = store.planes.iter().find(|(id, _)| *id == -2) {
        let warped = match mask {
            Some((m, mt)) if like.len() == (m.rect.width() * m.rect.height()) as usize => {
                let plane = unit_planes(&[like]).pop().unwrap_or_default();
                let default = m.default_color.to_unit() as f32;
                to_samples(like, warp_gray(plane, m.rect, default, mt, interp))
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
    let interp = interpolation.resolve((sw * sh) as u64, ((dr - dl) as u64) * ((db - dt) as u64));
    let alpha = prepared.source_channels.iter().position(|c| c.id == -1);
    let planes = prepared
        .source_channels
        .iter()
        .map(|c| c.data.iter().map(|v| v.to_unit() as f32).collect())
        .collect();
    let src = Planes::new(planes, sw, sh, alpha);
    let warped = warp_planes(&src, prepared.rect, &t, interp, false);
    let channels: Vec<Channel> = prepared
        .source_channels
        .iter()
        .zip(warped)
        .map(|(c, plane)| Channel {
            id: c.id,
            data: plane
                .into_iter()
                .map(|v| u8::from_unit(f64::from(v)))
                .collect::<Vec<u8>>()
                .into(),
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
    let out = LayerOutput {
        channels,
        mask: mask.map(|(m, _)| m),
        native,
        dest: t.dest,
    };
    write_layer(doc, path, &prepared, out, false)
}
