//! Filter ▸ Distort ▸ Displace.
//!
//! Ported from perfecto25/photorust (`core/src/filters/distort.rs`), with the
//! reach of the docs contract: at 100% the largest shift is 128 px, where
//! photorust moved a pixel half the image.

use super::distort::{remap, EdgeMode};
use super::pixmap::Pixmap;
use crate::{DisplaceFit, DisplaceMap};

/// Move every pixel by an amount read out of `map`.
///
/// The horizontal plane says how far to go sideways and the vertical plane how
/// far up or down: a sample `v` reaches `(v − 128) / 128 × scale% × 128` px,
/// so mid grey stays put. Each destination pixel reads the source that far
/// away, so a light map pulls the picture up and to the left. `fit` stretches
/// the map over the layer or tiles it at its own size; `wrap` chooses what a
/// pixel reaching past the edge picks up — the far side, or the edge repeated.
/// ponytail: the map covers the whole buffer (the layer), where CS6 fits it
/// to the selection's bounds.
pub fn displace(
    pixmap: &mut Pixmap,
    map: &DisplaceMap,
    horizontal_scale: f32,
    vertical_scale: f32,
    fit: DisplaceFit,
    wrap: bool,
) {
    // The shortcuts keep a do-nothing run bit-exact on translucent pixels,
    // where the premultiplied round trip in `remap` would not be.
    let neutral = map
        .horizontal
        .iter()
        .chain(&map.vertical)
        .all(|&v| v == 128);
    if pixmap.is_empty() || neutral || (horizontal_scale == 0.0 && vertical_scale == 0.0) {
        return;
    }
    let width = pixmap.width() as f32;
    let height = pixmap.height() as f32;
    let (map_w, map_h) = (map.width() as f32, map.height() as f32);
    let h = horizontal_scale / 100.0;
    let v = vertical_scale / 100.0;
    let edge = if wrap {
        EdgeMode::Wrap
    } else {
        EdgeMode::Clamp
    };

    remap(pixmap, edge, move |x, y| {
        let (across, down) = match fit {
            DisplaceFit::StretchToFit => {
                stretched(map, x / width * map_w - 0.5, y / height * map_h - 0.5)
            }
            DisplaceFit::Tile => {
                let (a, d) = map.at(x as u32 % map.width(), y as u32 % map.height());
                (f32::from(a), f32::from(d))
            }
        };
        (x + (across - 128.0) * h, y + (down - 128.0) * v)
    });
}

/// Both planes at map position `(u, v)`, bilinear and clamped, so a small map
/// stretched over a large layer bends smoothly instead of in steps.
fn stretched(map: &DisplaceMap, u: f32, v: f32) -> (f32, f32) {
    let (max_x, max_y) = (map.width() - 1, map.height() - 1);
    let u = u.clamp(0.0, max_x as f32);
    let v = v.clamp(0.0, max_y as f32);
    let (x0, y0) = (u as u32, v as u32);
    let (x1, y1) = ((x0 + 1).min(max_x), (y0 + 1).min(max_y));
    let (fx, fy) = (u - x0 as f32, v - y0 as f32);
    let corners = [
        map.at(x0, y0),
        map.at(x1, y0),
        map.at(x0, y1),
        map.at(x1, y1),
    ];
    let mix = |pick: fn((u8, u8)) -> u8| {
        let [a, b, c, d] = corners.map(|s| f32::from(pick(s)));
        let top = a + (b - a) * fx;
        let bottom = c + (d - c) * fx;
        top + (bottom - top) * fy
    };
    (mix(|s| s.0), mix(|s| s.1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photorust::pixmap::Rgba8;

    fn flat_map(width: u32, height: u32, horizontal: u8, vertical: u8) -> DisplaceMap {
        let n = (width * height) as usize;
        DisplaceMap::new(width, height, vec![horizontal; n], vec![vertical; n]).unwrap()
    }

    fn grid(size: u32, spacing: u32) -> Pixmap {
        let mut px = Pixmap::filled(size, size, Rgba8::WHITE);
        for y in 0..size as i32 {
            for x in 0..size as i32 {
                if (x as u32).is_multiple_of(spacing) || (y as u32).is_multiple_of(spacing) {
                    px.set(x, y, Rgba8::BLACK);
                }
            }
        }
        px
    }

    /// The first dark column on row `y`.
    fn dark_column(px: &Pixmap, y: i32) -> Option<i32> {
        (0..px.width() as i32).find(|&x| px.get(x, y).r < 128)
    }

    #[test]
    fn a_flat_grey_map_displaces_nothing_even_on_translucent_pixels() {
        // 128 means "stay put"; an off-by-one in the mid point would shift
        // the whole picture, and the premultiplied round trip would nudge
        // translucent colour.
        let mut source = grid(64, 8);
        source.set(3, 5, Rgba8::new(200, 90, 17, 41));
        let mut after = source.clone();
        let map = flat_map(64, 64, 128, 128);
        displace(
            &mut after,
            &map,
            100.0,
            100.0,
            DisplaceFit::StretchToFit,
            false,
        );
        assert_eq!(
            after.as_bytes(),
            source.as_bytes(),
            "a neutral map moved the image"
        );
        displace(
            &mut after,
            &DisplaceMap::neutral(),
            999.0,
            -999.0,
            DisplaceFit::Tile,
            true,
        );
        assert_eq!(
            after.as_bytes(),
            source.as_bytes(),
            "the neutral map moved the image"
        );
    }

    #[test]
    fn full_white_at_one_hundred_percent_reaches_127_pixels() {
        // The contract's reach: (255 − 128) / 128 × 128 px at 100%, toward
        // the light side, so the content moves left by 127.
        let mut source = Pixmap::filled(300, 4, Rgba8::WHITE);
        for y in 0..4 {
            source.set(200, y, Rgba8::BLACK);
        }
        let mut after = source.clone();
        displace(
            &mut after,
            &flat_map(1, 1, 255, 128),
            100.0,
            100.0,
            DisplaceFit::Tile,
            false,
        );
        assert_eq!(dark_column(&after, 1), Some(73));

        // Black reaches 128 the other way, and the scale is linear in it.
        let mut after = source.clone();
        displace(
            &mut after,
            &flat_map(1, 1, 0, 128),
            50.0,
            0.0,
            DisplaceFit::Tile,
            false,
        );
        assert_eq!(dark_column(&after, 1), Some(264));
    }

    #[test]
    fn the_map_planes_drive_the_two_axes_separately() {
        let source = grid(64, 8);
        let mut across = source.clone();
        displace(
            &mut across,
            &flat_map(8, 8, 255, 128),
            3.0,
            3.0,
            DisplaceFit::Tile,
            false,
        );
        let mut down = source.clone();
        displace(
            &mut down,
            &flat_map(8, 8, 128, 255),
            3.0,
            3.0,
            DisplaceFit::Tile,
            false,
        );
        assert_ne!(
            across.as_bytes(),
            source.as_bytes(),
            "the horizontal plane did nothing"
        );
        assert_ne!(
            down.as_bytes(),
            source.as_bytes(),
            "the vertical plane did nothing"
        );
        assert_ne!(
            across.as_bytes(),
            down.as_bytes(),
            "both planes moved the same way"
        );
        // A sideways shift keeps the rows where they were.
        assert_eq!(across.row(8), source.row(8));
    }

    #[test]
    fn one_plane_twice_shifts_along_the_diagonal_of_the_scales() {
        // A Grayscale map carries one plane for both axes, so the shift is
        // (h, v) × the same deflection.
        let mut source = Pixmap::filled(64, 64, Rgba8::WHITE);
        source.set(40, 40, Rgba8::BLACK);
        let mut after = source.clone();
        // 160 − 128 = 32, so 25% moves 8 px and 50% moves 16 px.
        displace(
            &mut after,
            &flat_map(1, 1, 160, 160),
            25.0,
            50.0,
            DisplaceFit::Tile,
            false,
        );
        assert!(
            after.get(32, 24).r < 128,
            "the dot did not land at (32, 24)"
        );
    }

    #[test]
    fn wrapping_brings_the_far_side_back_in() {
        let mut source = Pixmap::filled(64, 16, Rgba8::WHITE);
        for y in 0..16 {
            for x in 56..64 {
                source.set(x, y, Rgba8::BLACK);
            }
        }
        let map = flat_map(64, 16, 255, 128);
        let mut wrapped = source.clone();
        displace(
            &mut wrapped,
            &map,
            10.0,
            0.0,
            DisplaceFit::StretchToFit,
            true,
        );
        let mut repeated = source.clone();
        displace(
            &mut repeated,
            &map,
            10.0,
            0.0,
            DisplaceFit::StretchToFit,
            false,
        );
        // Reaching 12.7 px right off the end: Repeat Edge Pixels smears the
        // dark edge in, Wrap Around brings the white left side back.
        assert!(repeated.get(63, 8).r < 128);
        assert!(wrapped.get(63, 8).r > 128);
    }

    #[test]
    fn tile_repeats_the_map_where_stretch_spreads_it() {
        // A two-pixel map, dark then light, at a scale where dark reaches 4 px
        // left and light 3.97 px right. Stretched, the whole left edge reads
        // dark and the line at x = 5 lands at 9; tiled, the odd columns read
        // light and column 1 already reaches the line.
        let map = DisplaceMap::new(2, 1, vec![0, 255], vec![128, 128]).unwrap();
        let mut source = Pixmap::filled(64, 4, Rgba8::WHITE);
        for y in 0..4 {
            source.set(5, y, Rgba8::BLACK);
        }
        let mut stretched = source.clone();
        displace(
            &mut stretched,
            &map,
            3.125,
            0.0,
            DisplaceFit::StretchToFit,
            false,
        );
        assert_eq!(dark_column(&stretched, 1), Some(9));
        let mut tiled = source.clone();
        displace(&mut tiled, &map, 3.125, 0.0, DisplaceFit::Tile, false);
        assert_eq!(dark_column(&tiled, 1), Some(1));
    }
}
