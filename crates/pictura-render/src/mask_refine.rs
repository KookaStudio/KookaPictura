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
//! ponytail: the shared cache keeps the last [`CAPACITY`] feathered planes; a
//! document with more feathered masks than that rebuilds them every pass.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use pictura_core::{LayerMask, VectorMask};

/// The selection feather's maximum radius, which the blur clamps to.
const FEATHER_MAX: f64 = 250.0;
const CAPACITY: usize = 4;

/// The refined raster-mask sample at a document pixel: `255` when disabled
/// or with no decoded pixels.
pub(crate) fn raster(mask: &LayerMask, x: i32, y: i32) -> u8 {
    if mask.disabled {
        return 255;
    }
    let Some(data) = &mask.data else {
        return 255;
    };
    let radius = radius(mask.feather());
    let empty = mask.rect.width() <= 0 || mask.rect.height() <= 0;
    let m = if radius > 0.0 && !empty {
        let key = Key {
            source: data.ident(),
            rect: [
                mask.rect.left,
                mask.rect.top,
                mask.rect.right,
                mask.rect.bottom,
            ],
            outside: mask.default_color,
            feather: radius.to_bits(),
        };
        sample(key, x, y, || feather_raster(mask, data, radius))
    } else {
        crate::composite::mask_value(mask, x, y)
    };
    density(m, mask.density)
}

/// The refined vector-mask coverage at a document pixel: `255` for an absent,
/// disabled, or fill-less mask.
pub(crate) fn vector(mask: Option<&VectorMask>, x: i32, y: i32) -> u8 {
    let Some(mask) = mask.filter(|m| m.has_fill()) else {
        return 255;
    };
    let radius = radius(mask.feather());
    let m = if radius > 0.0 {
        let key = Key {
            source: mask.ident(),
            rect: [0; 4],
            outside: 0,
            feather: radius.to_bits(),
        };
        // Built uninverted: the blur is linear, so inverting after it is the
        // same as blurring the inverted coverage.
        let c = sample(key, x, y, || feather_vector(mask, radius));
        if mask.invert {
            255 - c
        } else {
            c
        }
    } else {
        crate::vector_mask::coverage(Some(mask), x, y)
    };
    density(m, mask.density)
}

/// Whether a mask with this density and feather samples differently from
/// its painted pixels.
pub(crate) fn is_refined(density: u8, feather: f64) -> bool {
    density != 255 || radius(feather) > 0.0
}

fn density(m: u8, d: u8) -> u8 {
    255 - ((u32::from(d) * u32::from(255 - m) + 127) / 255) as u8
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
fn reach(radius: f64) -> i32 {
    (1.5 * radius).ceil() as i32 + 1
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    source: u64,
    rect: [i32; 4],
    outside: u8,
    feather: u64,
}

/// A blurred coverage plane at a document offset; `outside` beyond it.
struct Feathered {
    left: i32,
    top: i32,
    width: usize,
    height: usize,
    outside: u8,
    data: Vec<u8>,
}

impl Feathered {
    fn at(&self, x: i32, y: i32) -> u8 {
        let (dx, dy) = (x - self.left, y - self.top);
        if dx < 0 || dy < 0 || dx as usize >= self.width || dy as usize >= self.height {
            return self.outside;
        }
        self.data[dy as usize * self.width + dx as usize]
    }

    /// Blur `data` (a `width` × `height` plane at `(left, top)`) in place.
    fn blurred(mut self, radius: f64) -> Feathered {
        let selection = pictura_select::Selection {
            width: self.width as u32,
            height: self.height as u32,
            data: std::mem::take(&mut self.data),
        };
        self.data = selection.feather(radius).data;
        self
    }
}

fn feather_raster(mask: &LayerMask, data: &[u8], radius: f64) -> Feathered {
    let pad = reach(radius);
    let (mw, mh) = (mask.rect.width() as usize, mask.rect.height() as usize);
    let width = mw + 2 * pad as usize;
    let height = mh + 2 * pad as usize;
    let mut plane = vec![mask.default_color; width * height];
    for (row, src) in data.chunks(mw).take(mh).enumerate() {
        let at = (row + pad as usize) * width + pad as usize;
        plane[at..at + src.len()].copy_from_slice(src);
    }
    Feathered {
        left: mask.rect.left - pad,
        top: mask.rect.top - pad,
        width,
        height,
        outside: mask.default_color,
        data: plane,
    }
    .blurred(radius)
}

fn feather_vector(mask: &VectorMask, radius: f64) -> Feathered {
    let points = mask
        .subpaths
        .iter()
        .filter(|s| s.closed)
        .flat_map(|s| &s.points);
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for p in points {
        x0 = x0.min(p[0].div_euclid(256));
        y0 = y0.min(p[1].div_euclid(256));
        x1 = x1.max(p[0].div_euclid(256) + 1);
        y1 = y1.max(p[1].div_euclid(256) + 1);
    }
    let pad = reach(radius);
    let (left, top) = (x0 - pad, y0 - pad);
    let width = (x1 - x0 + 2 * pad) as usize;
    let height = (y1 - y0 + 2 * pad) as usize;
    let mut plane = vec![0u8; width * height];
    for (row, line) in plane.chunks_exact_mut(width).enumerate() {
        for (col, v) in line.iter_mut().enumerate() {
            if mask.inside(left + col as i32, top + row as i32) {
                *v = 255;
            }
        }
    }
    Feathered {
        left,
        top,
        width,
        height,
        outside: 0,
        data: plane,
    }
    .blurred(radius)
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
        assert_eq!(raster(&mask, 0, 0), 0, "full density is the painted mask");
        mask.density = 128;
        assert_eq!(raster(&mask, 0, 0), 127);
        assert_eq!(raster(&mask, 3, 0), 255, "white stays white");
        mask.density = 0;
        assert_eq!(raster(&mask, 0, 0), 255, "zero density reveals everything");
        mask.disabled = true;
        mask.density = 255;
        assert_eq!(raster(&mask, 0, 0), 255, "a disabled mask still reveals");
    }

    #[test]
    fn feather_ramps_a_hard_edge_monotonically() {
        let mut mask = step(64, 0);
        mask.set_feather(8.0);
        // The edge is at x = 32; the rect's own edges (default colour 0) are
        // more than a ramp's width away from 20..44.
        let row: Vec<u8> = (20..44).map(|x| raster(&mask, x, 32)).collect();
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
            raster(&mask, 16, 8) > 0,
            "the white side bleeds past the rect"
        );
        assert_eq!(
            raster(&mask, 200, 8),
            0,
            "far outside is the default colour"
        );
    }

    #[test]
    fn painting_the_mask_retires_its_feathered_plane() {
        let mut mask = step(32, 0);
        mask.set_feather(4.0);
        let before = raster(&mask, 4, 16);
        assert_eq!(before, 0);
        mask.data.as_mut().unwrap().as_mut_slice().fill(255);
        assert_eq!(
            raster(&mask, 4, 16),
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
            vector(Some(&mask), 31, 16),
            255,
            "a hard edge without feather"
        );
        mask.set_feather(6.0);
        let inside_edge = vector(Some(&mask), 31, 16);
        assert!(inside_edge > 0 && inside_edge < 255, "{inside_edge}");
        assert!(vector(Some(&mask), 33, 16) > 0, "the ramp crosses the path");
        assert_eq!(vector(Some(&mask), 16, 16), 255);
        mask.invert = true;
        assert_eq!(vector(Some(&mask), 31, 16), 255 - inside_edge);
        mask.invert = false;
        mask.density = 0;
        assert_eq!(vector(Some(&mask), 100, 100), 255);
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
}
