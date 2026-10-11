//! Custom mesh warp: a layer resampled through a tensor-product cubic Bézier
//! surface defined by a control net, plus the options-bar distortion.
//!
//! `TOOL-001` (`docs/03-tools/move-and-transform.md`) marks the exact
//! patch degree and the named preset parameters closed; this ships the exact,
//! self-verifiable core (a *custom* net, identity-exact) and leaves the presets
//! to a change that can pin them against a reference. No reference pixel oracle
//! exists for the warp surface, so parity is behavioral.
//!
//! The op reuses [`prepare_layer`] / [`write_layer`] from `transform.rs`, so its
//! refusal, channel-less materialization, mask, and smart-object rules match the
//! similarity and projective ops.

use pictura_core::{Channel, Document, LayerMask, PsdRect};

use super::kernel::{self, Interpolation};
use super::transform::{integer_bbox, prepare_layer, write_layer, LayerOutput};
use super::transform_native;

/// Number of parameter cells per axis the surface is rasterized through.
const CELLS: usize = 16;

/// A row-major control net in the source rect's local space.
#[derive(Debug, Clone, PartialEq)]
pub struct WarpMesh {
    pub rows: usize,
    pub cols: usize,
    pub points: Vec<(f64, f64)>,
}

impl WarpMesh {
    /// An empty net with no points yet; preset constructions append rows.
    pub(crate) fn new(cols: usize, rows: usize) -> WarpMesh {
        WarpMesh {
            rows,
            cols,
            points: Vec::new(),
        }
    }
}

/// Options-bar distortion in percent.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WarpParams {
    pub distort_h: f64,
    pub distort_v: f64,
}

/// The uniform control net over a `w × h` rect; the identity warp.
pub fn identity_mesh(cols: usize, rows: usize, w: i32, h: i32) -> WarpMesh {
    let (cd, rd) = ((cols - 1).max(1) as f64, (rows - 1).max(1) as f64);
    let mut points = Vec::with_capacity(rows * cols);
    for j in 0..rows {
        for i in 0..cols {
            points.push((i as f64 / cd * w as f64, j as f64 / rd * h as f64));
        }
    }
    WarpMesh { rows, cols, points }
}

/// Binomial coefficient `C(n, k)` for the small degrees used here.
fn binom(n: usize, k: usize) -> f64 {
    let mut result = 1.0f64;
    for i in 0..k {
        result = result * (n - i) as f64 / (i + 1) as f64;
    }
    result
}

/// Bernstein basis of degree `n` at `t`.
fn bernstein(t: f64, n: usize) -> Vec<f64> {
    (0..=n)
        .map(|k| binom(n, k) * t.powi(k as i32) * (1.0 - t).powi((n - k) as i32))
        .collect()
}

/// The Bézier surface point for `(u, v) ∈ [0, 1]²`, in local space.
fn surface(mesh: &WarpMesh, u: f64, v: f64) -> (f64, f64) {
    let bu = bernstein(u, mesh.cols - 1);
    let bv = bernstein(v, mesh.rows - 1);
    let mut x = 0.0;
    let mut y = 0.0;
    for (j, bvj) in bv.iter().enumerate() {
        for (i, bui) in bu.iter().enumerate() {
            let p = mesh.points[j * mesh.cols + i];
            let weight = bui * bvj;
            x += p.0 * weight;
            y += p.1 * weight;
        }
    }
    (x, y)
}

/// Apply the options-bar distortion: scale each net row about its edge-point
/// midpoint, then each column likewise. Inferred from `SethRobinson/Patchy`'s
/// read of the reference's distortion operator.
fn distorted(mesh: &WarpMesh, params: WarpParams) -> WarpMesh {
    let mut m = mesh.clone();
    let (rows, cols) = (mesh.rows, mesh.cols);
    for j in 0..rows {
        let v = if rows > 1 {
            j as f64 / (rows - 1) as f64
        } else {
            0.0
        };
        let s = 1.0 + (2.0 * v - 1.0) * params.distort_v / 100.0;
        let a = m.points[j * cols];
        let b = m.points[j * cols + cols - 1];
        let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        for i in 0..cols {
            let p = &mut m.points[j * cols + i];
            p.0 = mid.0 + (p.0 - mid.0) * s;
            p.1 = mid.1 + (p.1 - mid.1) * s;
        }
    }
    for i in 0..cols {
        let u = if cols > 1 {
            i as f64 / (cols - 1) as f64
        } else {
            0.0
        };
        let s = 1.0 + (2.0 * u - 1.0) * params.distort_h / 100.0;
        let a = m.points[i];
        let b = m.points[(rows - 1) * cols + i];
        let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        for j in 0..rows {
            let p = &mut m.points[j * cols + i];
            p.0 = mid.0 + (p.0 - mid.0) * s;
            p.1 = mid.1 + (p.1 - mid.1) * s;
        }
    }
    m
}

/// Barycentric coordinates of `p` in triangle `(a, b, c)`, or `None` when it is
/// degenerate.
fn barycentric(p: (f64, f64), a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> Option<[f64; 3]> {
    let v0 = (b.0 - a.0, b.1 - a.1);
    let v1 = (c.0 - a.0, c.1 - a.1);
    let v2 = (p.0 - a.0, p.1 - a.1);
    let den = v0.0 * v1.1 - v1.0 * v0.1;
    if den.abs() < 1e-12 {
        return None;
    }
    let beta = (v2.0 * v1.1 - v1.0 * v2.1) / den;
    let gamma = (v0.0 * v2.1 - v2.0 * v0.1) / den;
    Some([1.0 - beta - gamma, beta, gamma])
}

/// Resample one plane through the warp surface.
///
/// `to_src(u, v)` maps the surface parameter to the sampled plane's source-local
/// coordinate. Cells paint in forward order, so a self-intersecting mesh lets a
/// later cell overwrite an earlier one. `sample` is the plane's bilinear kernel
/// (`transform::bilinear` for `u8`, the native analogue otherwise).
pub(super) fn warp_plane<T: Copy + Default>(
    src: &[T],
    src_w: usize,
    src_h: usize,
    mesh: &WarpMesh,
    to_src: impl Fn(f64, f64) -> (f64, f64),
    dest: (i32, i32, i32, i32),
    sample: impl Fn(&[T], usize, usize, f64, f64) -> T,
) -> Vec<T> {
    let (dl, dt, dr, db) = dest;
    let dw = (dr - dl) as usize;
    let dh = (db - dt) as usize;
    let mut out = vec![T::default(); dw * dh];
    let wf = src_w as f64;
    let hf = src_h as f64;
    for cj in 0..CELLS {
        for ci in 0..CELLS {
            let u0 = ci as f64 / CELLS as f64;
            let u1 = (ci + 1) as f64 / CELLS as f64;
            let v0 = cj as f64 / CELLS as f64;
            let v1 = (cj + 1) as f64 / CELLS as f64;
            let p00 = surface(mesh, u0, v0);
            let p10 = surface(mesh, u1, v0);
            let p11 = surface(mesh, u1, v1);
            let p01 = surface(mesh, u0, v1);
            let xs = [p00.0, p10.0, p11.0, p01.0];
            let ys = [p00.1, p10.1, p11.1, p01.1];
            let min_x = xs.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_x = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let min_y = ys.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_y = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let x0 = ((dl as f64 + min_x).floor() as i64).max(dl as i64).max(0);
            let x1 = ((dl as f64 + max_x).ceil() as i64).min(dr as i64 - 1);
            let y0 = ((dt as f64 + min_y).floor() as i64).max(dt as i64).max(0);
            let y1 = ((dt as f64 + max_y).ceil() as i64).min(db as i64 - 1);
            for oy in y0..=y1 {
                for ox in x0..=x1 {
                    let q = (ox as f64 + 0.5, oy as f64 + 0.5);
                    let inside = |b: [f64; 3]| b[0] >= -1e-6 && b[1] >= -1e-6 && b[2] >= -1e-6;
                    // Cell (a, b) in [0,1]²: triangle 1 is (0,0),(1,0),(1,1);
                    // triangle 2 is (0,0),(1,1),(0,1).
                    let tri1 = barycentric(q, p00, p10, p11)
                        .filter(|b| inside(*b))
                        .map(|b| (b[1] + b[2], b[2]));
                    let cell_ab = tri1.or_else(|| {
                        barycentric(q, p00, p11, p01)
                            .filter(|b| inside(*b))
                            .map(|b| (b[1], b[1] + b[2]))
                    });
                    let Some((a, b)) = cell_ab else { continue };
                    let u = u0 + a * (u1 - u0);
                    let v = v0 + b * (v1 - v0);
                    let (sx, sy) = to_src(u, v);
                    if sx >= 0.0 && sy >= 0.0 && sx < wf && sy < hf {
                        let idx = (oy - dt as i64) as usize * dw + (ox - dl as i64) as usize;
                        out[idx] = sample(src, src_w, src_h, sx, sy);
                    }
                }
            }
        }
    }
    out
}

/// The transformed mask, or `None` when its plane size disagrees with its rect.
fn warp_mask(
    mask: &LayerMask,
    mesh: &WarpMesh,
    layer_rect: PsdRect,
    interpolation: Interpolation,
) -> Option<LayerMask> {
    let rect = mask.rect;
    let (w, h) = (rect.width(), rect.height());
    if w <= 0 || h <= 0 {
        return None;
    }
    let lw = layer_rect.width() as f64;
    let lh = layer_rect.height() as f64;
    let mut pts = [(0.0, 0.0); 4];
    for (out, (mx, my)) in pts.iter_mut().zip([
        (rect.left as f64, rect.top as f64),
        (rect.right as f64, rect.top as f64),
        (rect.right as f64, rect.bottom as f64),
        (rect.left as f64, rect.bottom as f64),
    ]) {
        let u = (mx - layer_rect.left as f64) / lw;
        let v = (my - layer_rect.top as f64) / lh;
        let (lx, ly) = surface(mesh, u, v);
        *out = (layer_rect.left as f64 + lx, layer_rect.top as f64 + ly);
    }
    let dest = integer_bbox(&pts)?;
    let resolved = interpolation.resolve(
        (w as u64) * (h as u64),
        ((dest.2 - dest.0) as u64) * ((dest.3 - dest.1) as u64),
    );
    let data = match mask.data.as_ref() {
        Some(data) => {
            if data.len() != (w as usize) * (h as usize) {
                return None;
            }
            let ox = layer_rect.left as f64 - rect.left as f64;
            let oy = layer_rect.top as f64 - rect.top as f64;
            Some(warp_plane(
                data,
                w as usize,
                h as usize,
                mesh,
                move |u, v| (u * lw + ox, v * lh + oy),
                dest,
                |src, sw, sh, lx, ly| kernel::sample(src, sw, sh, lx, ly, resolved),
            ))
        }
        None => None,
    };
    Some(LayerMask {
        rect: PsdRect {
            top: dest.1,
            left: dest.0,
            bottom: dest.3,
            right: dest.2,
        },
        data: data.map(Into::into),
        ..mask.clone()
    })
}

/// Apply a custom mesh warp to the layer at `path`.
///
/// Refuses (leaving `doc` bit-identical) for every case `transform_layer`
/// refuses, plus a non-finite mesh point, `rows`/`cols` below 2, or a point
/// count other than `rows · cols`. `pure_translate` is always false: a warp
/// drops an unmodeled raw channel stream.
pub fn transform_layer_warp(
    doc: &mut Document,
    path: &str,
    mesh: &WarpMesh,
    params: WarpParams,
) -> bool {
    transform_layer_warp_with(doc, path, mesh, params, Interpolation::Bilinear)
}

/// Apply a custom mesh warp with an explicit resample `interpolation`. Same
/// contract as [`transform_layer_warp`].
pub fn transform_layer_warp_with(
    doc: &mut Document,
    path: &str,
    mesh: &WarpMesh,
    params: WarpParams,
    interpolation: Interpolation,
) -> bool {
    if mesh.rows < 2
        || mesh.cols < 2
        || mesh.points.len() != mesh.rows * mesh.cols
        || mesh
            .points
            .iter()
            .any(|p| !p.0.is_finite() || !p.1.is_finite())
        || !params.distort_h.is_finite()
        || !params.distort_v.is_finite()
    {
        return false;
    }
    let Some(prepared) = prepare_layer(doc, path) else {
        return false;
    };
    let m = distorted(mesh, params);
    let mut lattice = Vec::with_capacity((CELLS + 1) * (CELLS + 1));
    for j in 0..=CELLS {
        for i in 0..=CELLS {
            let (lx, ly) = surface(&m, i as f64 / CELLS as f64, j as f64 / CELLS as f64);
            lattice.push((
                prepared.rect.left as f64 + lx,
                prepared.rect.top as f64 + ly,
            ));
        }
    }
    let Some(dest) = integer_bbox(&lattice) else {
        return false;
    };
    let sw = prepared.w as usize;
    let sh = prepared.h as usize;
    let resolved = interpolation.resolve(
        (sw as u64) * (sh as u64),
        ((dest.2 - dest.0) as u64) * ((dest.3 - dest.1) as u64),
    );
    let new_channels: Vec<Channel> = prepared
        .source_channels
        .iter()
        .map(|c| Channel {
            id: c.id,
            data: warp_plane(
                &c.data,
                sw,
                sh,
                &m,
                |u, v| (u * sw as f64, v * sh as f64),
                dest,
                |src, w, h, lx, ly| kernel::sample(src, w, h, lx, ly, resolved),
            )
            .into(),
        })
        .collect();
    let new_mask = prepared
        .mask
        .as_ref()
        .and_then(|mk| warp_mask(mk, &m, prepared.rect, interpolation));
    let native = prepared.native.as_ref().map(|store| {
        let mask_info = prepared
            .mask
            .as_ref()
            .zip(new_mask.as_ref())
            .map(|(mk, nmk)| (mk.rect, transform_native::dest_tuple(nmk.rect)));
        transform_native::warp_store(
            store,
            sw,
            sh,
            &m,
            prepared.rect,
            dest,
            mask_info,
            interpolation,
        )
    });
    let out = LayerOutput {
        channels: new_channels,
        mask: new_mask,
        native,
        dest,
    };
    write_layer(doc, path, &prepared, out, false)
}

#[cfg(test)]
mod tests {
    use super::super::paths::resolve_path;
    use super::*;
    use pictura_core::{BitDepth, ColorMode, Layer, Samples, SourceChannels, SourcePlanes};

    fn rect(w: i32, h: i32) -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: h,
            right: w,
        }
    }

    fn plane(w: u32, h: u32, seed: u8) -> Vec<u8> {
        (0..(w * h) as usize)
            .map(|i| (i as u8).wrapping_mul(7).wrapping_add(seed))
            .collect()
    }

    fn pixel_layer(w: u32, h: u32) -> Layer {
        let n = (w * h) as usize;
        Layer {
            name: "L".into(),
            rect: rect(w as i32, h as i32),
            channels: vec![
                Channel {
                    id: 0,
                    data: plane(w, h, 3).into(),
                },
                Channel {
                    id: 1,
                    data: plane(w, h, 11).into(),
                },
                Channel {
                    id: 2,
                    data: plane(w, h, 29).into(),
                },
                Channel {
                    id: -1,
                    data: vec![255; n].into(),
                },
            ],
            ..Default::default()
        }
    }

    fn doc_with(layer: Layer) -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = vec![layer];
        doc
    }

    #[test]
    fn identity_mesh_is_a_noop() {
        let mut doc = doc_with(pixel_layer(4, 4));
        let before = doc.clone();
        let mesh = identity_mesh(4, 4, 4, 4);
        assert!(transform_layer_warp(
            &mut doc,
            "0",
            &mesh,
            WarpParams::default()
        ));
        assert_eq!(doc, before);
    }

    #[test]
    fn warp_interpolation_reaches_the_plane() {
        let mut nearest = doc_with(pixel_layer(9, 9));
        let mut bilinear = doc_with(pixel_layer(9, 9));
        let mut mesh = identity_mesh(4, 4, 9, 9);
        mesh.points[5] = (2.0, 2.0);
        assert!(transform_layer_warp_with(
            &mut nearest,
            "0",
            &mesh,
            WarpParams::default(),
            Interpolation::Nearest
        ));
        assert!(transform_layer_warp(
            &mut bilinear,
            "0",
            &mesh,
            WarpParams::default()
        ));
        assert_ne!(nearest, bilinear);
    }

    #[test]
    fn moved_interior_point_deforms_but_keeps_corners() {
        let mut doc = doc_with(pixel_layer(9, 9));
        let before = doc.clone();
        let mut mesh = identity_mesh(4, 4, 9, 9);
        // Interior control point (row 1, col 1) at (3,3) pulls inward.
        mesh.points[5] = (2.0, 2.0);
        assert!(transform_layer_warp(
            &mut doc,
            "0",
            &mesh,
            WarpParams::default()
        ));
        assert_ne!(doc, before, "interior must be resampled");
        let layer = resolve_path(&doc, "0").unwrap();
        assert_eq!((layer.rect.width(), layer.rect.height()), (9, 9));
        let plane = 9 * 9;
        let alpha = layer.channels.iter().find(|c| c.id == -1).unwrap();
        assert_eq!(alpha.data.len(), plane);
        assert_eq!(alpha.data[0], 255, "top-left corner stays opaque");
        assert_eq!(
            alpha.data[plane - 1],
            255,
            "bottom-right corner stays opaque"
        );
    }

    #[test]
    fn degenerate_meshes_are_refused() {
        let mut doc = doc_with(pixel_layer(4, 4));
        let before = doc.clone();
        let mut bad = identity_mesh(4, 4, 4, 4);
        bad.points[5] = (f64::NAN, 0.0);
        assert!(!transform_layer_warp(
            &mut doc,
            "0",
            &bad,
            WarpParams::default()
        ));
        assert_eq!(doc, before);
        let mut wrong = identity_mesh(4, 4, 4, 4);
        wrong.points.pop();
        assert!(!transform_layer_warp(
            &mut doc,
            "0",
            &wrong,
            WarpParams::default()
        ));
        assert_eq!(doc, before);
    }

    #[test]
    fn out_of_source_pixels_are_zero() {
        // A single interior control point pulled far outside the rect bends the
        // surface so its destination bounding box has corners the sheet never
        // reaches; those pixels stay transparent.
        let mut doc = doc_with(pixel_layer(8, 8));
        let mut mesh = identity_mesh(4, 4, 8, 8);
        mesh.points[5] = (30.0, 30.0);
        assert!(transform_layer_warp(
            &mut doc,
            "0",
            &mesh,
            WarpParams::default()
        ));
        let layer = resolve_path(&doc, "0").unwrap();
        let alpha = layer.channels.iter().find(|c| c.id == -1).unwrap();
        assert!(
            alpha.data.contains(&0),
            "an uncovered destination pixel is transparent"
        );
    }

    #[test]
    fn distortion_scales_rows_before_columns() {
        // Asymmetric net so the two axis scalings do not commute: distort_v runs
        // over rows first, then distort_h over columns (Patchy's `so_persp_*`
        // order).
        let mut mesh = identity_mesh(3, 3, 9, 9);
        mesh.points[0] = (2.0, 3.0);
        mesh.points[4] = (2.0, 7.0);
        let both = WarpParams {
            distort_h: 60.0,
            distort_v: 40.0,
        };
        let vertical_first = distorted(
            &distorted(
                &mesh,
                WarpParams {
                    distort_h: 0.0,
                    distort_v: both.distort_v,
                },
            ),
            WarpParams {
                distort_h: both.distort_h,
                distort_v: 0.0,
            },
        );
        let horizontal_first = distorted(
            &distorted(
                &mesh,
                WarpParams {
                    distort_h: both.distort_h,
                    distort_v: 0.0,
                },
            ),
            WarpParams {
                distort_h: 0.0,
                distort_v: both.distort_v,
            },
        );
        let close = |a: &WarpMesh, b: &WarpMesh, tol: f64| {
            a.points.len() == b.points.len()
                && a.points
                    .iter()
                    .zip(&b.points)
                    .all(|(p, q)| (p.0 - q.0).abs() < tol && (p.1 - q.1).abs() < tol)
        };
        assert!(
            close(&distorted(&mesh, both), &vertical_first, 1e-9),
            "distortion must run rows (vertical) before columns (horizontal)"
        );
        assert!(
            !close(&distorted(&mesh, both), &horizontal_first, 1e-6),
            "the reverse order is a different mapping on an asymmetric net"
        );
    }

    #[test]
    fn depth16_warp_resamples_and_keeps_the_native_store() {
        let mut layer = pixel_layer(8, 8);
        let n = 64;
        let native = |seed: u8| {
            Samples::U16(
                plane(8, 8, seed)
                    .into_iter()
                    .map(|v| (v as u16 * 257).saturating_add(7))
                    .collect(),
            )
        };
        layer.source_channels = Some(SourceChannels::new(
            BitDepth::Sixteen,
            layer.rect,
            vec![
                (0, native(3)),
                (1, native(11)),
                (2, native(29)),
                (-1, Samples::U16(vec![65535; n])),
            ],
        ));
        let mut doc = doc_with(layer);
        doc.source_depth = Some(BitDepth::Sixteen);
        doc.source_planes = Some(SourcePlanes {
            depth: BitDepth::Sixteen,
            width: 8,
            height: 8,
            samples: Samples::U16(vec![0; 3 * n]),
        });

        let mut mesh = identity_mesh(4, 4, 8, 8);
        mesh.points[5] = (2.0, 2.0);
        assert!(transform_layer_warp(
            &mut doc,
            "0",
            &mesh,
            WarpParams::default()
        ));
        let layer = resolve_path(&doc, "0").unwrap();
        let store = layer
            .source_channels
            .as_ref()
            .expect("a warp keeps a resampled native store");
        assert_eq!(store.rect, layer.rect);
        let area = (layer.rect.width() * layer.rect.height()) as usize;
        assert_eq!(
            store
                .planes
                .iter()
                .find(|(id, _)| *id == 0)
                .unwrap()
                .1
                .len(),
            area
        );
        let bytes = pictura_codec::write_psd(&doc).expect("a warped high-depth layer saves");
        assert_eq!(u16::from_be_bytes(bytes[22..24].try_into().unwrap()), 16);
    }
}
