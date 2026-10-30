//! Mask density and feather (the Properties panel's mask refinements).
//!
//! Both are inferred, Adobe's are closed: density lifts the coverage toward
//! white (`m' = 255 - d (255 - m) / 255`), and feather is the selection
//! feather's Gaussian (`sigma = radius / 2`, clamped at 250 px) over the mask's
//! coverage, whose ramp runs past a raster mask's rect into its default colour.
//!
//! A feathered plane is built once per mask state and sampled per pixel. The
//! state is named by the source's ident ([`pictura_core::Plane::ident`] or
//! [`VectorMask::ident`]), so painting the mask retires its plane without any
//! hook. Lookups go through a small per-thread cache in front of a bounded
//! shared one, so a hit takes no lock.
//!
//! The plane is cut to the caller's `clip` (the pixels it samples) grown by
//! the feather's reach: a mask's rect and path come from the file, so they
//! must not size the allocation. Samples inside `clip` are exact.
//!
//! ponytail: the shared cache keeps the last [`CAPACITY`] feathered planes; a
//! document with more feathered masks than that rebuilds them every pass.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use pictura_core::{LayerMask, Plane, PsdRect, Samples, VectorMask};

/// The selection feather's maximum radius, which the blur clamps to.
const FEATHER_MAX: f64 = 250.0;
const CAPACITY: usize = 4;

/// The document canvas, the `clip` for samples taken over it.
#[cfg(test)]
pub(crate) fn canvas(doc: &pictura_core::Document) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height.min(i32::MAX as u32) as i32,
        right: doc.width.min(i32::MAX as u32) as i32,
    }
}

/// The refined raster-mask sample at a document pixel inside `clip`: `255`
/// when disabled or with no decoded pixels.
pub(crate) fn raster(mask: &LayerMask, clip: PsdRect, x: i32, y: i32) -> u8 {
    if mask.disabled {
        return 255;
    }
    let Some(data) = &mask.data else {
        return 255;
    };
    let radius = radius(mask.feather());
    let r = mask.rect;
    let empty = i64::from(r.right) <= i64::from(r.left) || i64::from(r.bottom) <= i64::from(r.top);
    let m = if radius > 0.0 && !empty {
        let key = Key {
            source: data.ident(),
            rect: [
                mask.rect.left,
                mask.rect.top,
                mask.rect.right,
                mask.rect.bottom,
            ],
            clip: rect_key(clip),
            outside: mask.default_color,
            feather: radius.to_bits(),
        };
        sample(key, x, y, || feather_raster(mask, data, radius, clip))
    } else {
        crate::composite::mask_value(mask, x, y)
    };
    density(m, mask.density)
}

/// The refined vector-mask coverage at a document pixel inside `clip`: `255`
/// for an absent, disabled, or fill-less mask.
pub(crate) fn vector(mask: Option<&VectorMask>, clip: PsdRect, x: i32, y: i32) -> u8 {
    let Some(mask) = mask.filter(|m| m.has_fill()) else {
        return 255;
    };
    let radius = radius(mask.feather());
    let m = if radius > 0.0 {
        let key = Key {
            source: mask.ident(),
            rect: [0; 4],
            clip: rect_key(clip),
            outside: 0,
            feather: radius.to_bits(),
        };
        // Built uninverted: the blur is linear, so inverting after it is the
        // same as blurring the inverted coverage.
        let c = sample(key, x, y, || feather_vector(mask, radius, clip));
        if mask.invert {
            255 - c
        } else {
            c
        }
    } else {
        crate::vector_mask::coverage(Some(mask), clip, x, y)
    };
    density(m, mask.density)
}

/// Whether a mask with this density and feather samples differently from
/// its painted pixels.
pub(crate) fn is_refined(density: u8, feather: f64) -> bool {
    density != 255 || radius(feather) > 0.0
}

fn density(m: u8, d: u8) -> u8 {
    if d == 255 {
        return m;
    }
    255 - ((u32::from(d) * u32::from(255 - m) + 127) / 255) as u8
}

/// A mask's raster part resolved for a whole layer pass: the plain plane, a
/// feathered plane built once, or a high-depth native `-2` plane.
pub(crate) enum RasterGate<'a> {
    None,
    Plain {
        plane: &'a Plane<u8>,
        rect: PsdRect,
        default_color: u8,
        density: u8,
    },
    Feathered {
        plane: Arc<OnceLock<Feathered>>,
        density: u8,
    },
    Native {
        samples: &'a Samples,
        rect: PsdRect,
        default_color: u8,
    },
}

impl RasterGate<'_> {
    /// The 8-bit raster sample (density applied), or `255` when the mask
    /// reveals the layer. For `Native`, this is the out-of-rect fallback.
    pub(crate) fn u8_at(&self, x: i32, y: i32) -> u8 {
        match self {
            RasterGate::None => 255,
            RasterGate::Plain {
                plane,
                rect,
                default_color,
                density,
            } => plain_mask_at(plane, *rect, *default_color, *density, x, y),
            RasterGate::Feathered { plane, density: d } => {
                let m = plane
                    .get()
                    .expect("the feathered plane is resolved eagerly")
                    .at(x, y);
                density(m, *d)
            }
            RasterGate::Native { default_color, .. } => *default_color,
        }
    }

    /// The native `-2` sample in unit space, when the pixel is inside the
    /// native mask rect; `None` keeps the 8-bit fallback.
    pub(crate) fn native_at(&self, x: i32, y: i32) -> Option<f32> {
        let RasterGate::Native { samples, rect, .. } = self else {
            return None;
        };
        native_mask_sample(samples, *rect, x, y)
    }
}

/// The mask's vector part resolved for a whole layer pass.
pub(crate) enum VectorGate<'a> {
    None,
    Coverage {
        coverage: pictura_core::VectorCoverage<'a>,
        invert: bool,
        density: u8,
    },
    Feathered {
        plane: Arc<OnceLock<Feathered>>,
        invert: bool,
        density: u8,
    },
}

impl VectorGate<'_> {
    pub(crate) fn u8_at(&self, x: i32, y: i32) -> u8 {
        match self {
            VectorGate::None => 255,
            VectorGate::Coverage {
                coverage,
                invert,
                density: d,
            } => {
                let c = coverage.level(x, y);
                let c = if *invert { 255 - c } else { c };
                density(c, *d)
            }
            VectorGate::Feathered {
                plane,
                invert,
                density: d,
            } => {
                let c = plane
                    .get()
                    .expect("the feathered plane is resolved eagerly")
                    .at(x, y);
                let c = if *invert { 255 - c } else { c };
                density(c, *d)
            }
        }
    }
}

/// The layer's raster mask resolved once for `clip`.
pub(crate) fn resolve_raster(mask: Option<&LayerMask>, clip: PsdRect) -> RasterGate<'_> {
    let Some(mask) = mask else {
        return RasterGate::None;
    };
    if mask.disabled {
        return RasterGate::None;
    }
    let Some(data) = mask.data.as_ref() else {
        return RasterGate::None;
    };
    let radius = radius(mask.feather());
    let empty = i64::from(mask.rect.right) <= i64::from(mask.rect.left)
        || i64::from(mask.rect.bottom) <= i64::from(mask.rect.top);
    if radius > 0.0 && !empty {
        let key = Key {
            source: data.ident(),
            rect: [
                mask.rect.left,
                mask.rect.top,
                mask.rect.right,
                mask.rect.bottom,
            ],
            clip: rect_key(clip),
            outside: mask.default_color,
            feather: radius.to_bits(),
        };
        let plane = shared_cell(key);
        plane.get_or_init(|| feather_raster(mask, data, radius, clip));
        RasterGate::Feathered {
            plane,
            density: mask.density,
        }
    } else {
        RasterGate::Plain {
            plane: data,
            rect: mask.rect,
            default_color: mask.default_color,
            density: mask.density,
        }
    }
}

/// The layer's vector mask resolved once for `clip`.
pub(crate) fn resolve_vector(mask: Option<&VectorMask>, clip: PsdRect) -> VectorGate<'_> {
    let Some(mask) = mask.filter(|m| m.has_fill()) else {
        return VectorGate::None;
    };
    let radius = radius(mask.feather());
    if radius > 0.0 {
        let key = Key {
            source: mask.ident(),
            rect: [0; 4],
            clip: rect_key(clip),
            outside: 0,
            feather: radius.to_bits(),
        };
        let plane = shared_cell(key);
        plane.get_or_init(|| feather_vector(mask, radius, clip));
        VectorGate::Feathered {
            plane,
            invert: mask.invert,
            density: mask.density,
        }
    } else {
        VectorGate::Coverage {
            coverage: mask.coverage_sampler(clip),
            invert: mask.invert,
            density: mask.density,
        }
    }
}

fn plain_mask_at(plane: &Plane<u8>, rect: PsdRect, default_color: u8, d: u8, x: i32, y: i32) -> u8 {
    let mw = rect.width();
    let mh = rect.height();
    let m = if mw <= 0 || mh <= 0 {
        default_color
    } else {
        let mx = x - rect.left;
        let my = y - rect.top;
        if mx < 0 || my < 0 || mx >= mw || my >= mh {
            default_color
        } else {
            let idx = my as usize * mw as usize + mx as usize;
            plane.get(idx).copied().unwrap_or(default_color)
        }
    };
    density(m, d)
}

fn native_mask_sample(samples: &Samples, rect: PsdRect, x: i32, y: i32) -> Option<f32> {
    let mw = rect.width();
    let mh = rect.height();
    if mw <= 0 || mh <= 0 {
        return None;
    }
    let mx = x - rect.left;
    let my = y - rect.top;
    if mx < 0 || my < 0 || mx >= mw || my >= mh {
        return None;
    }
    samples.row(my as usize, mw as usize)?.unit(mx as usize)
}

/// The usable feather radius: `0` for none, negative, or non-finite.
fn radius(feather: f64) -> f64 {
    if feather.is_finite() && feather > 0.0 {
        feather.min(FEATHER_MAX)
    } else {
        0.0
    }
}

/// How far a feather of `radius` spreads: three standard deviations.
pub(crate) fn reach(radius: f64) -> i32 {
    (1.5 * radius).ceil() as i32 + 1
}

/// How far a mask's feather can spread a changed sample, or 0 without a
/// feather. Used to grow a mask stroke's paint refresh rectangle.
pub fn mask_feather_reach(mask: &LayerMask) -> i32 {
    if mask.disabled {
        return 0;
    }
    let radius = radius(mask.feather());
    if radius > 0.0 {
        reach(radius)
    } else {
        0
    }
}

/// Test-only: the extent of the feathered plane a raster mask would build for
/// `clip`, so a test can assert a region build does not cover the whole mask.
#[cfg(test)]
pub(crate) fn raster_plane_bounds(mask: &LayerMask, clip: PsdRect) -> Option<[i32; 4]> {
    let data = mask.data.as_ref()?;
    let radius = radius(mask.feather());
    if radius <= 0.0 {
        return None;
    }
    let plane = feather_raster(mask, data, radius, clip);
    Some([
        plane.left,
        plane.top,
        plane.left + plane.width as i32,
        plane.top + plane.height as i32,
    ])
}

fn rect_key(r: PsdRect) -> [i32; 4] {
    [r.left, r.top, r.right, r.bottom]
}

/// `bounds ∩ clip`, both grown by `pad`, as `[left, top, right, bottom]`;
/// empty when they miss.
fn plane_bounds(bounds: [i64; 4], clip: PsdRect, pad: i64) -> [i32; 4] {
    let c = [clip.left, clip.top, clip.right, clip.bottom].map(i64::from);
    let lo = |i: usize| (bounds[i].max(c[i]) - pad).max(i64::from(i32::MIN)) as i32;
    let hi = |i: usize| (bounds[i].min(c[i]) + pad).min(i64::from(i32::MAX)) as i32;
    let [left, top, right, bottom] = [lo(0), lo(1), hi(2), hi(3)];
    [left, top, right.max(left), bottom.max(top)]
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    source: u64,
    rect: [i32; 4],
    clip: [i32; 4],
    outside: u8,
    feather: u64,
}

/// A blurred coverage plane at a document offset; `outside` beyond it.
pub(crate) struct Feathered {
    left: i32,
    top: i32,
    width: usize,
    height: usize,
    outside: u8,
    data: Vec<u8>,
}

impl Feathered {
    /// An unblurred plane over `[left, top, right, bottom]` filled with `fill`.
    fn new(bounds: [i32; 4], fill: u8, outside: u8) -> Feathered {
        let [left, top, right, bottom] = bounds;
        let width = (i64::from(right) - i64::from(left)) as usize;
        let height = (i64::from(bottom) - i64::from(top)) as usize;
        Feathered {
            left,
            top,
            width,
            height,
            outside,
            data: vec![fill; width * height],
        }
    }

    fn at(&self, x: i32, y: i32) -> u8 {
        let dx = i64::from(x) - i64::from(self.left);
        let dy = i64::from(y) - i64::from(self.top);
        if dx < 0 || dy < 0 || dx as usize >= self.width || dy as usize >= self.height {
            return self.outside;
        }
        self.data[dy as usize * self.width + dx as usize]
    }

    /// Blur `data` (a `width` × `height` plane at `(left, top)`) in place.
    fn blurred(mut self, radius: f64) -> Feathered {
        if self.data.is_empty() {
            return self;
        }
        let selection = pictura_select::Selection {
            width: self.width as u32,
            height: self.height as u32,
            data: std::mem::take(&mut self.data).into(),
        };
        self.data = selection.feather(radius).data.to_vec();
        self
    }
}

fn feather_raster(mask: &LayerMask, data: &[u8], radius: f64, clip: PsdRect) -> Feathered {
    let r = mask.rect;
    let rect = [r.left, r.top, r.right, r.bottom].map(i64::from);
    let pad = i64::from(reach(radius));
    let bounds = plane_bounds(rect, clip, pad);
    let mut plane = Feathered::new(bounds, mask.default_color, mask.default_color);
    let mw = (rect[2] - rect[0]) as usize;
    // The mask's columns and rows that land in the plane.
    let x0 = i64::from(bounds[0]).max(rect[0]);
    let x1 = i64::from(bounds[2]).min(rect[2]);
    let y0 = i64::from(bounds[1]).max(rect[1]);
    let y1 = i64::from(bounds[3]).min(rect[3]);
    if x1 > x0 {
        for y in y0..y1 {
            let src = (y - rect[1]) as usize * mw + (x0 - rect[0]) as usize;
            let Some(src) = data.get(src..src + (x1 - x0) as usize) else {
                break;
            };
            let at = (y - i64::from(plane.top)) as usize * plane.width
                + (x0 - i64::from(plane.left)) as usize;
            plane.data[at..at + src.len()].copy_from_slice(src);
        }
    }
    plane.blurred(radius)
}

fn feather_vector(mask: &VectorMask, radius: f64, clip: PsdRect) -> Feathered {
    let points = mask
        .subpaths
        .iter()
        .filter(|s| s.closed)
        .flat_map(|s| &s.points);
    let mut bbox = [i64::MAX, i64::MAX, i64::MIN, i64::MIN];
    for p in points {
        let (x, y) = (
            i64::from(p[0].div_euclid(256)),
            i64::from(p[1].div_euclid(256)),
        );
        bbox = [
            bbox[0].min(x),
            bbox[1].min(y),
            bbox[2].max(x + 1),
            bbox[3].max(y + 1),
        ];
    }
    let bounds = plane_bounds(bbox, clip, i64::from(reach(radius)));
    // The plane is the exact region sampled, so it bounds the row table too.
    let plane_rect = PsdRect {
        left: bounds[0],
        top: bounds[1],
        right: bounds[2],
        bottom: bounds[3],
    };
    let mut plane = Feathered::new(bounds, 0, 0);
    if plane.width > 0 {
        for (row, line) in plane.data.chunks_exact_mut(plane.width).enumerate() {
            for (col, v) in line.iter_mut().enumerate() {
                if mask.inside(bounds[0] + col as i32, bounds[1] + row as i32, plane_rect) {
                    *v = 255;
                }
            }
        }
    }
    plane.blurred(radius)
}

type Entry = (Key, Arc<OnceLock<Feathered>>);

/// Bumped whenever the shared cache changes, so per-thread caches drop
/// entries the shared one no longer pins.
static GENERATION: AtomicU64 = AtomicU64::new(0);
static SHARED: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

thread_local! {
    static LOCAL: RefCell<(u64, Vec<Entry>)> = const { RefCell::new((0, Vec::new())) };
}

fn sample(key: Key, x: i32, y: i32, build: impl FnOnce() -> Feathered) -> u8 {
    LOCAL.with(|local| {
        let mut local = local.borrow_mut();
        let generation = GENERATION.load(Ordering::Acquire);
        if local.0 != generation {
            *local = (generation, Vec::new());
        }
        let i = match local.1.iter().position(|(k, _)| *k == key) {
            Some(i) => i,
            None => {
                local.1.push((key, shared_cell(key)));
                local.1.len() - 1
            }
        };
        // The build runs outside every lock; threads asking for the same key
        // wait on its cell instead of building it twice.
        local.1[i].1.get_or_init(build).at(x, y)
    })
}

fn shared_cell(key: Key) -> Arc<OnceLock<Feathered>> {
    let mut shared = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(i) = shared.iter().position(|(k, _)| *k == key) {
        let entry = shared.remove(i);
        let cell = Arc::clone(&entry.1);
        shared.push(entry);
        return cell;
    }
    let cell = Arc::new(OnceLock::new());
    shared.push((key, Arc::clone(&cell)));
    if shared.len() > CAPACITY {
        shared.remove(0);
    }
    GENERATION.fetch_add(1, Ordering::AcqRel);
    cell
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{PsdRect, VectorFillRule, VectorSubpath};

    const CANVAS: PsdRect = PsdRect {
        top: 0,
        left: 0,
        bottom: 256,
        right: 256,
    };

    /// A `w`×`w` raster mask at the origin: black left half, white right half.
    fn step(w: i32, default_color: u8) -> LayerMask {
        LayerMask {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: w,
                right: w,
            },
            default_color,
            data: Some(
                (0..w * w)
                    .map(|i| if i % w < w / 2 { 0 } else { 255 })
                    .collect::<Vec<u8>>()
                    .into(),
            ),
            ..Default::default()
        }
    }

    #[test]
    fn density_lifts_the_mask_toward_white() {
        let mut mask = step(4, 0);
        assert_eq!(
            raster(&mask, CANVAS, 0, 0),
            0,
            "full density is the painted mask"
        );
        mask.density = 128;
        assert_eq!(raster(&mask, CANVAS, 0, 0), 127);
        assert_eq!(raster(&mask, CANVAS, 3, 0), 255, "white stays white");
        mask.density = 0;
        assert_eq!(
            raster(&mask, CANVAS, 0, 0),
            255,
            "zero density reveals everything"
        );
        mask.disabled = true;
        mask.density = 255;
        assert_eq!(
            raster(&mask, CANVAS, 0, 0),
            255,
            "a disabled mask still reveals"
        );
    }

    #[test]
    fn feather_ramps_a_hard_edge_monotonically() {
        let mut mask = step(64, 0);
        mask.set_feather(8.0);
        // The edge is at x = 32; the rect's own edges (default colour 0) are
        // more than a ramp's width away from 20..44.
        let row: Vec<u8> = (20..44).map(|x| raster(&mask, CANVAS, x, 32)).collect();
        assert!(row.windows(2).all(|w| w[0] <= w[1]), "{row:?}");
        assert!(row[12] > 0 && row[12] < 255, "the edge softens: {row:?}");
        assert_eq!(row[0], 0, "far from the edge stays black");
        assert_eq!(row[23], 255, "far from the edge stays white");
    }

    #[test]
    fn feather_reaches_past_the_rect_into_the_default_colour() {
        let mut mask = step(16, 0);
        mask.set_feather(8.0);
        assert!(
            raster(&mask, CANVAS, 16, 8) > 0,
            "the white side bleeds past the rect"
        );
        assert_eq!(
            raster(&mask, CANVAS, 200, 8),
            0,
            "far outside is the default colour"
        );
    }

    #[test]
    fn painting_the_mask_retires_its_feathered_plane() {
        let mut mask = step(32, 0);
        mask.set_feather(4.0);
        let before = raster(&mask, CANVAS, 4, 16);
        assert_eq!(before, 0);
        mask.data.as_mut().unwrap().as_mut_slice().fill(255);
        assert_eq!(
            raster(&mask, CANVAS, 4, 16),
            255,
            "the cache follows the new pixels"
        );
    }

    #[test]
    fn a_feathered_vector_mask_softens_and_inverts() {
        let p = |x: i32, y: i32| [x * 256, y * 256];
        let mut mask = VectorMask {
            subpaths: vec![VectorSubpath {
                closed: true,
                operation: 1,
                fill_rule: VectorFillRule::EvenOdd,
                points: vec![p(0, 0), p(32, 0), p(32, 32), p(0, 32)],
            }],
            ..Default::default()
        };
        assert_eq!(
            vector(Some(&mask), CANVAS, 31, 16),
            255,
            "a hard edge without feather"
        );
        mask.set_feather(6.0);
        let inside_edge = vector(Some(&mask), CANVAS, 31, 16);
        assert!(inside_edge > 0 && inside_edge < 255, "{inside_edge}");
        assert!(
            vector(Some(&mask), CANVAS, 33, 16) > 0,
            "the ramp crosses the path"
        );
        assert_eq!(vector(Some(&mask), CANVAS, 16, 16), 255);
        mask.invert = true;
        assert_eq!(vector(Some(&mask), CANVAS, 31, 16), 255 - inside_edge);
        mask.invert = false;
        mask.density = 0;
        assert_eq!(vector(Some(&mask), CANVAS, 100, 100), 255);
    }

    #[test]
    fn file_bounds_far_off_the_canvas_do_not_size_the_plane() {
        // Unclipped, the raster plane would be ~2^64 bytes and the vector one
        // ~16 GB (the path is kept short so its row table stays small).
        let mut mask = LayerMask {
            rect: PsdRect {
                top: i32::MIN,
                left: i32::MIN,
                bottom: i32::MAX,
                right: i32::MAX,
            },
            default_color: 0,
            data: Some(vec![255u8; 4].into()),
            ..Default::default()
        };
        mask.set_feather(4.0);
        assert_eq!(
            raster(&mask, CANVAS, 10, 10),
            0,
            "past the data is the default"
        );
        let p = |x: i32, y: i32| [x, y];
        let mut path = VectorMask {
            subpaths: vec![VectorSubpath {
                closed: true,
                operation: 1,
                fill_rule: VectorFillRule::EvenOdd,
                points: vec![
                    p(i32::MIN, 0),
                    p(i32::MAX, 0),
                    p(i32::MAX, 256 * 1000),
                    p(i32::MIN, 256 * 1000),
                ],
            }],
            ..Default::default()
        };
        path.set_feather(4.0);
        assert_eq!(vector(Some(&path), CANVAS, 128, 128), 255);
    }

    #[test]
    fn a_mask_off_the_canvas_still_feathers_onto_its_edge() {
        let mut mask = step(16, 0);
        mask.rect.left = 256;
        mask.rect.right = 272;
        mask.data.as_mut().unwrap().as_mut_slice().fill(255);
        mask.set_feather(8.0);
        assert!(
            raster(&mask, CANVAS, 255, 8) > 0,
            "the ramp crosses the clip"
        );
        assert_eq!(raster(&mask, CANVAS, 100, 8), 0);
    }

    #[test]
    fn unusable_feathers_are_none() {
        for f in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(!is_refined(255, f), "{f}");
        }
        assert!(is_refined(255, 0.5));
        assert!(is_refined(254, 0.0));
        assert_eq!(radius(1000.0), FEATHER_MAX);
    }

    #[test]
    fn a_resolved_raster_gate_matches_the_per_pixel_raster() {
        let mut mask = step(16, 0);
        let plain = resolve_raster(Some(&mask), CANVAS);
        for y in 0..16 {
            for x in 0..16 {
                assert_eq!(
                    plain.u8_at(x, y),
                    raster(&mask, CANVAS, x, y),
                    "plain ({x}, {y})"
                );
            }
        }
        mask.set_feather(4.0);
        let feathered = resolve_raster(Some(&mask), CANVAS);
        for y in 0..16 {
            for x in 0..16 {
                assert_eq!(
                    feathered.u8_at(x, y),
                    raster(&mask, CANVAS, x, y),
                    "feathered ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn a_resolved_vector_gate_matches_the_per_pixel_vector() {
        let p = |x: i32, y: i32| [x * 256, y * 256];
        let mut mask = VectorMask {
            subpaths: vec![VectorSubpath {
                closed: true,
                operation: 1,
                fill_rule: VectorFillRule::EvenOdd,
                points: vec![p(0, 0), p(32, 0), p(32, 32), p(0, 32)],
            }],
            ..Default::default()
        };
        let plain = resolve_vector(Some(&mask), CANVAS);
        for y in 0..32 {
            for x in 0..32 {
                assert_eq!(
                    plain.u8_at(x, y),
                    vector(Some(&mask), CANVAS, x, y),
                    "plain ({x}, {y})"
                );
            }
        }
        mask.set_feather(6.0);
        let feathered = resolve_vector(Some(&mask), CANVAS);
        for y in 0..32 {
            for x in 0..32 {
                assert_eq!(
                    feathered.u8_at(x, y),
                    vector(Some(&mask), CANVAS, x, y),
                    "feathered ({x}, {y})"
                );
            }
        }
    }
}
