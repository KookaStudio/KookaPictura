//! Decode a layer's preserved `vmsk` block into the derived
//! [`pictura_core::VectorMask`] view, and author one from a Work Path outline.
//!
//! The raw block stays in `Layer.extra_blocks` and is the source of truth for
//! re-emission; this module only parses and flattens it. A malformed block
//! leaves the view unset rather than failing the document.

use pictura_core::path::{PathPoint, Subpath};
use pictura_core::{Layer, VectorFillRule, VectorMask, VectorSubpath};

use crate::common::Reader;

/// 8.24 fixed-point denominator: `0x01000000` maps to the document extent.
const FIXED_ONE: f64 = 0x0100_0000 as f64;
/// Every path record is a 2-byte selector plus a 24-byte payload.
const RECORD: usize = 26;
/// Cubics are flattened at a fixed subdivision; see `flatten`.
const SUBDIVISIONS: u32 = 16;

/// Resolve `vmsk` for `layers` (and their children) against the document size.
/// Never fails: an absent or unparseable block leaves `vector_mask` unset.
pub(crate) fn resolve_vector_masks(layers: &mut [Layer], width: u32, height: u32) {
    for layer in layers.iter_mut() {
        layer.vector_mask = layer
            .extra_block(b"vmsk")
            .and_then(|block| decode(&block.data, width, height));
        resolve_vector_masks(&mut layer.children, width, height);
    }
}

/// Decode a `vmsk` block against the document size; `None` when malformed.
pub fn decode_vector_mask(data: &[u8], width: u32, height: u32) -> Option<VectorMask> {
    decode(data, width, height)
}

/// Decode a `vmsk` block's subpaths as editable Bezier anchors in document
/// pixels: a handle that sits on its anchor reads as absent, and a linked knot
/// is smooth. `None` when malformed.
pub fn decode_vector_mask_paths(data: &[u8], width: u32, height: u32) -> Option<Vec<Subpath>> {
    let mut r = Reader::new(data);
    if r.u32().ok()? != 3 {
        return None;
    }
    r.u32().ok()?;
    decode_path_records(data.get(8..)?, width, height)
}

/// Decode bare path records (a `vmsk` body, or a path image resource's data)
/// as editable subpaths in document pixels; `None` when malformed.
pub(crate) fn decode_path_records(data: &[u8], width: u32, height: u32) -> Option<Vec<Subpath>> {
    let mut r = Reader::new(data);
    let px = |raw: i32, extent: u32| raw as f64 / FIXED_ONE * extent as f64;
    let mut subpaths = Vec::new();
    while r.remaining() >= RECORD {
        match r.u16().ok()? {
            selector @ (0 | 3) => {
                let count = r.u16().ok()? as usize;
                r.skip(22).ok()?;
                let mut points = Vec::with_capacity(count);
                for _ in 0..count {
                    let kind = r.u16().ok()?;
                    if !matches!(kind, 1 | 2 | 4 | 5) {
                        return None;
                    }
                    let mut xy = [(0.0, 0.0); 3];
                    for slot in &mut xy {
                        let y = r.i32().ok()?;
                        let x = r.i32().ok()?;
                        *slot = (px(x, width), px(y, height));
                    }
                    let [preceding, anchor, leaving] = xy;
                    let handle = |h: (f64, f64)| {
                        ((h.0 - anchor.0).abs() > 1e-6 || (h.1 - anchor.1).abs() > 1e-6)
                            .then_some(h)
                    };
                    points.push(PathPoint {
                        anchor,
                        in_handle: handle(preceding),
                        out_handle: handle(leaving),
                        smooth: matches!(kind, 1 | 4),
                    });
                }
                subpaths.push(Subpath {
                    points,
                    closed: selector == 0,
                });
            }
            1 | 2 | 4 | 5 => return None,
            6..=8 => r.skip(24).ok()?,
            _ => return None,
        }
    }
    Some(subpaths)
}

/// Author a version-3 `vmsk` block (no flags) holding `subpaths`: a leading
/// path-fill-rule record, then per subpath a length record (operation 1,
/// union) and its knots, linked where the point is smooth. Coordinates are
/// 8.24 fractions of the document extent, `(y, x)` as the format orders them.
pub fn encode_vector_mask(subpaths: &[Subpath], width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + RECORD * (1 + subpaths.len() * 5));
    out.extend_from_slice(&3u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    encode_path_records(subpaths, width, height, &mut out);
    out
}

/// Append the bare path records `encode_vector_mask` frames: the fill-rule
/// record, then each subpath's length record and knots.
pub(crate) fn encode_path_records(
    subpaths: &[Subpath],
    width: u32,
    height: u32,
    out: &mut Vec<u8>,
) {
    out.extend_from_slice(&6u16.to_be_bytes());
    out.extend_from_slice(&[0u8; 24]);
    let fixed = |v: f64, extent: u32| ((v / extent.max(1) as f64) * FIXED_ONE).round() as i32;
    for subpath in subpaths {
        let selector: u16 = if subpath.closed { 0 } else { 3 };
        out.extend_from_slice(&selector.to_be_bytes());
        out.extend_from_slice(&(subpath.points.len() as u16).to_be_bytes());
        out.extend_from_slice(&1i16.to_be_bytes());
        out.extend_from_slice(&[0u8; 20]);
        let (linked, unlinked): (u16, u16) = if subpath.closed { (1, 2) } else { (4, 5) };
        for p in &subpath.points {
            let kind = if p.smooth { linked } else { unlinked };
            out.extend_from_slice(&kind.to_be_bytes());
            for (x, y) in [
                p.in_handle.unwrap_or(p.anchor),
                p.anchor,
                p.out_handle.unwrap_or(p.anchor),
            ] {
                out.extend_from_slice(&fixed(y, height).to_be_bytes());
                out.extend_from_slice(&fixed(x, width).to_be_bytes());
            }
        }
    }
}

/// Author a version-3 `vmsk` block from an already-flattened
/// [`VectorSubpath`] view, preserving each subpath's operation and fill rule.
/// The polyline vertices are written as corner knots; this is the inverse of
/// [`decode`] over the serializable fields, used when an in-session geometry
/// operation (e.g. rotation) mutates the view and the authoritative raw block
/// must be kept in sync.
pub fn encode_vector_mask_view(subpaths: &[VectorSubpath], width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + RECORD * (1 + subpaths.len() * 5));
    out.extend_from_slice(&3u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&6u16.to_be_bytes());
    out.extend_from_slice(&[0u8; 24]);
    let fixed = |v: f64, extent: u32| ((v / extent.max(1) as f64) * FIXED_ONE).round() as i32;
    for subpath in subpaths {
        let selector: u16 = if subpath.closed { 0 } else { 3 };
        out.extend_from_slice(&selector.to_be_bytes());
        out.extend_from_slice(&(subpath.points.len() as u16).to_be_bytes());
        out.extend_from_slice(&subpath.operation.to_be_bytes());
        let fill: u16 = match subpath.fill_rule {
            VectorFillRule::NonZero => 2,
            VectorFillRule::EvenOdd => 0,
        };
        out.extend_from_slice(&fill.to_be_bytes());
        out.extend_from_slice(&[0u8; 18]);
        let knot: u16 = if subpath.closed { 2 } else { 5 };
        for p in &subpath.points {
            out.extend_from_slice(&knot.to_be_bytes());
            let y = fixed(p[1] as f64 / 256.0, height);
            let x = fixed(p[0] as f64 / 256.0, width);
            for _ in 0..3 {
                out.extend_from_slice(&y.to_be_bytes());
                out.extend_from_slice(&x.to_be_bytes());
            }
        }
    }
    out
}

/// One knot in 1/256-pixel document coordinates.
struct Knot {
    preceding: [i32; 2],
    anchor: [i32; 2],
    leaving: [i32; 2],
}

fn decode(data: &[u8], width: u32, height: u32) -> Option<VectorMask> {
    let mut r = Reader::new(data);
    if r.u32().ok()? != 3 {
        return None;
    }
    let flags = r.u32().ok()?;
    let mut mask = VectorMask {
        subpaths: Vec::new(),
        invert: flags & 0x01 != 0,
        disabled: flags & 0x04 != 0,
        rows: Default::default(),
    };
    while r.remaining() >= RECORD {
        let selector = r.u16().ok()?;
        match selector {
            0 | 3 => {
                let closed = selector == 0;
                let count = r.u16().ok()? as usize;
                let operation = r.i16().ok()?;
                let fill = r.u16().ok()?;
                r.skip(18).ok()?;
                let mut knots = Vec::with_capacity(count);
                for _ in 0..count {
                    if r.remaining() < RECORD {
                        return None;
                    }
                    if !matches!(r.u16().ok()?, 1 | 2 | 4 | 5) {
                        return None;
                    }
                    knots.push(read_knot(&mut r, width, height)?);
                }
                mask.subpaths.push(flatten(closed, operation, fill, &knots));
            }
            // A knot or subpath-tail record before any subpath is malformed.
            1 | 2 | 4 | 5 => return None,
            6 | 7 => r.skip(24).ok()?,
            8 => {
                r.u16().ok()?;
                r.skip(22).ok()?;
            }
            _ => return None,
        }
    }
    Some(mask)
}

fn read_knot(r: &mut Reader, width: u32, height: u32) -> Option<Knot> {
    Some(Knot {
        preceding: read_point(r, width, height)?,
        anchor: read_point(r, width, height)?,
        leaving: read_point(r, width, height)?,
    })
}

/// Read one `(y, x)` 8.24 pair and return it as 1/256-pixel `[x, y]`.
fn read_point(r: &mut Reader, width: u32, height: u32) -> Option<[i32; 2]> {
    let y = r.i32().ok()?;
    let x = r.i32().ok()?;
    Some([scale(x, width), scale(y, height)])
}

fn scale(raw: i32, extent: u32) -> i32 {
    (raw as f64 / FIXED_ONE * extent as f64 * 256.0).round() as i32
}

/// Flatten a subpath's Bezier knots into a closed polyline.
///
/// `ponytail:` a fixed 16 segments per cubic; the result is exact for polygonal
/// masks and faceted on tight curves. Upgrade to adaptive flatness if curved
/// vector masks show visible faceting.
fn flatten(closed: bool, operation: i16, fill: u16, knots: &[Knot]) -> VectorSubpath {
    let fill_rule = if fill == 2 {
        VectorFillRule::NonZero
    } else {
        VectorFillRule::EvenOdd
    };
    let mut points = Vec::new();
    let n = knots.len();
    if n > 0 {
        points.push(knots[0].anchor);
        let segments = if closed { n } else { n - 1 };
        for i in 0..segments {
            let from = &knots[i];
            let to = &knots[(i + 1) % n];
            // A straight segment (handles on their anchors) is one edge.
            if from.leaving == from.anchor && to.preceding == to.anchor {
                points.push(to.anchor);
                continue;
            }
            for step in 1..=SUBDIVISIONS {
                let t = step as f64 / SUBDIVISIONS as f64;
                points.push(cubic(from.anchor, from.leaving, to.preceding, to.anchor, t));
            }
        }
    }
    VectorSubpath {
        closed,
        operation,
        fill_rule,
        points,
    }
}

/// Evaluate one cubic Bezier segment (integer 1/256-pixel controls).
fn cubic(p0: [i32; 2], p1: [i32; 2], p2: [i32; 2], p3: [i32; 2], t: f64) -> [i32; 2] {
    let u = 1.0 - t;
    let mut out = [0i32; 2];
    for c in 0..2 {
        let a = p0[c] as f64;
        let b = p1[c] as f64;
        let d = p2[c] as f64;
        let e = p3[c] as f64;
        out[c] = (u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * d + t * t * t * e).round()
            as i32;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::LayerBlock;

    fn mask_layer(data: Vec<u8>) -> Layer {
        Layer {
            extra_blocks: vec![LayerBlock {
                key: *b"vmsk",
                data,
            }],
            ..Default::default()
        }
    }

    fn resolve(data: Vec<u8>) -> Option<VectorMask> {
        let mut layer = mask_layer(data);
        resolve_vector_masks(std::slice::from_mut(&mut layer), 8, 8);
        layer.vector_mask
    }

    fn header(version: u32, flags: u32) -> Vec<u8> {
        let mut v = version.to_be_bytes().to_vec();
        v.extend_from_slice(&flags.to_be_bytes());
        v
    }

    #[test]
    fn rectangle_decodes_to_a_closed_subpath() {
        // One closed subpath with 4 corner knots at (0,0)-(6,6) in an 8x8 doc.
        let mut data = header(3, 0);
        data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(&4u16.to_be_bytes()); // knot count
        data.extend_from_slice(&1i16.to_be_bytes()); // operation
        data.extend_from_slice(&1u16.to_be_bytes()); // fill-rule field
        data.extend_from_slice(&[0u8; 18]);
        for (y, x) in [(0u32, 0u32), (0, 6), (6, 6), (6, 0)] {
            data.extend_from_slice(&1u16.to_be_bytes());
            for value in [
                ((y as f64 / 8.0) * FIXED_ONE) as i32,
                ((x as f64 / 8.0) * FIXED_ONE) as i32,
            ]
            .repeat(3)
            {
                data.extend_from_slice(&value.to_be_bytes());
            }
        }
        data.extend_from_slice(&6u16.to_be_bytes());
        data.extend_from_slice(&[0u8; 24]);

        let mask = resolve(data).expect("decodes");
        assert_eq!(mask.subpaths.len(), 1);
        let sub = &mask.subpaths[0];
        assert!(sub.closed);
        assert_eq!(sub.operation, 1);
        assert_eq!(sub.fill_rule, VectorFillRule::EvenOdd);
        assert!(sub.points.contains(&[0, 0]));
        assert!(sub.points.contains(&[6 * 256, 6 * 256]));
    }

    #[test]
    fn an_authored_mask_decodes_to_the_same_outline() {
        let square = pictura_core::shape::outline(
            pictura_core::shape::ShapeOptions::new(
                pictura_core::shape::ShapeKind::Rectangle,
                0.0,
                3,
            ),
            (2.0, 1.0),
            (6.0, 7.0),
            false,
            false,
        )
        .unwrap();
        let data = encode_vector_mask(&[square], 8, 8);
        let mask = decode_vector_mask(&data, 8, 8).expect("decodes");
        assert!(!mask.invert && !mask.disabled);
        assert_eq!(mask.subpaths.len(), 1);
        let sub = &mask.subpaths[0];
        assert!(sub.closed);
        assert_eq!(sub.operation, 1);
        assert_eq!(sub.fill_rule, VectorFillRule::EvenOdd);
        for corner in [[2, 1], [6, 1], [6, 7], [2, 7]] {
            assert!(sub.points.contains(&[corner[0] * 256, corner[1] * 256]));
        }
    }

    #[test]
    fn authored_curves_decode_back_to_the_same_anchors() {
        let options =
            pictura_core::shape::ShapeOptions::new(pictura_core::shape::ShapeKind::Ellipse, 0.0, 3);
        let ellipse =
            pictura_core::shape::outline_in_box(options, (16.0, 32.0, 64.0, 32.0)).unwrap();
        let data = encode_vector_mask(std::slice::from_ref(&ellipse), 128, 128);
        let back = decode_vector_mask_paths(&data, 128, 128).expect("decodes");
        assert_eq!(back.len(), 1);
        assert!(back[0].closed);
        for (a, b) in back[0].points.iter().zip(&ellipse.points) {
            assert!(a.smooth && b.smooth);
            let close =
                |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).abs() < 1e-4 && (p.1 - q.1).abs() < 1e-4;
            assert!(close(a.anchor, b.anchor));
            assert!(close(a.in_handle.unwrap(), b.in_handle.unwrap()));
            assert!(close(a.out_handle.unwrap(), b.out_handle.unwrap()));
        }
        assert_eq!(decode_vector_mask_paths(&[0, 0, 0, 2], 8, 8), None);
    }

    #[test]
    fn view_encoder_preserves_operation_fill_and_vertices() {
        let subpaths = vec![VectorSubpath {
            closed: true,
            operation: 3,
            fill_rule: VectorFillRule::NonZero,
            points: vec![[0, 0], [512, 0], [512, 512]],
        }];
        let data = encode_vector_mask_view(&subpaths, 8, 8);
        let mask = decode_vector_mask(&data, 8, 8).expect("decodes");
        let sub = &mask.subpaths[0];
        assert!(sub.closed);
        assert_eq!(sub.operation, 3);
        assert_eq!(sub.fill_rule, VectorFillRule::NonZero);
        assert!(sub.points.contains(&[512, 512]));
    }

    #[test]
    fn flags_set_invert_and_disable() {
        assert!(resolve(header(3, 0x01)).unwrap().invert);
        assert!(resolve(header(3, 0x04)).unwrap().disabled);
        assert!(!resolve(header(3, 0x02)).unwrap().invert);
    }

    #[test]
    fn fill_rule_marker_two_is_non_zero() {
        let mut data = header(3, 0);
        data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes()); // zero knots
        data.extend_from_slice(&1i16.to_be_bytes());
        data.extend_from_slice(&2u16.to_be_bytes()); // ag-psd marker
        data.extend_from_slice(&[0u8; 18]);
        let mask = resolve(data).unwrap();
        assert_eq!(mask.subpaths[0].fill_rule, VectorFillRule::NonZero);
    }

    #[test]
    fn fill_rule_field_zero_is_even_odd() {
        let mut data = header(3, 0);
        data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes()); // zero knots
        data.extend_from_slice(&1i16.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes()); // explicit even-odd field
        data.extend_from_slice(&[0u8; 18]);
        let mask = resolve(data).unwrap();
        assert_eq!(mask.subpaths[0].fill_rule, VectorFillRule::EvenOdd);
    }

    #[test]
    fn malformed_blocks_are_unset() {
        assert!(resolve(header(2, 0)).is_none(), "bad version");
        assert!(
            resolve(header(3, 0)).is_some_and(|m| m.subpaths.is_empty()),
            "a header with no records is an empty mask, not malformed"
        );

        let mut unknown = header(3, 0);
        unknown.extend_from_slice(&9u16.to_be_bytes());
        unknown.extend_from_slice(&[0u8; 24]);
        assert!(resolve(unknown).is_none(), "unknown selector");

        let mut knot_without_subpath = header(3, 0);
        knot_without_subpath.extend_from_slice(&1u16.to_be_bytes());
        knot_without_subpath.extend_from_slice(&[0u8; 24]);
        assert!(
            resolve(knot_without_subpath).is_none(),
            "knot without subpath"
        );

        let mut truncated = header(3, 0);
        truncated.extend_from_slice(&0u16.to_be_bytes());
        truncated.extend_from_slice(&4u16.to_be_bytes()); // claims 4 knots
        truncated.extend_from_slice(&1i16.to_be_bytes());
        truncated.extend_from_slice(&1u16.to_be_bytes());
        truncated.extend_from_slice(&[0u8; 18]);
        assert!(resolve(truncated).is_none(), "missing knots");
    }
}
