use super::*;

/// A straight-alpha composite with something different in every pixel.
struct Built {
    r: Vec<u8>,
    g: Vec<u8>,
    b: Vec<u8>,
    a: Vec<u8>,
    width: u32,
    height: u32,
}

impl Built {
    fn new(width: u32, height: u32, seed: u32) -> Self {
        let n = width as usize * height as usize;
        let mut r = vec![0u8; n];
        let mut g = vec![0u8; n];
        let mut b = vec![0u8; n];
        let mut a = vec![0u8; n];
        for y in 0..height {
            for x in 0..width {
                let i = y as usize * width as usize + x as usize;
                let v = (x * 7 + y * 13 + seed) % 251;
                r[i] = v as u8;
                g[i] = (v / 2) as u8;
                b[i] = (255 - v) as u8;
                a[i] = (60 + (x * 3 + y) % 196) as u8;
            }
        }
        Self {
            r,
            g,
            b,
            a,
            width,
            height,
        }
    }

    fn planes(&self) -> Planes<'_> {
        Planes::new(self.width, self.height, &self.r, &self.g, &self.b, &self.a)
    }
}

#[test]
fn levels_halve_down_to_the_smallest_side() {
    let p = Built::new(1500, 700, 0);
    let pyramid = ViewPyramid::rebuild(p.planes());
    let sizes: Vec<_> = (0..pyramid.level_count())
        .map(|l| pyramid.level_size(l))
        .collect();
    assert_eq!(sizes, vec![(1500, 700), (750, 350), (375, 175)]);
}

#[test]
fn a_511_long_side_still_gets_a_256_level() {
    let p = Built::new(511, 300, 0);
    let pyramid = ViewPyramid::rebuild(p.planes());
    let sizes: Vec<_> = (0..pyramid.level_count())
        .map(|l| pyramid.level_size(l))
        .collect();
    assert_eq!(sizes, vec![(511, 300), (256, 150)]);
}

#[test]
fn a_small_composite_is_its_only_level() {
    let p = Built::new(300, 200, 0);
    assert_eq!(ViewPyramid::rebuild(p.planes()).level_count(), 1);
}

#[test]
fn odd_edges_average_over_what_is_there() {
    let p = Built::new(1031, 517, 3);
    let pyramid = ViewPyramid::rebuild(p.planes());
    let above = &pyramid.levels[0];
    for (x, y) in [(0, 0), (100, 50), (515, 258), (515, 0), (0, 258)] {
        let mut sum = [0u32; 4];
        let mut n = 0u32;
        for dy in 0..2 {
            for dx in 0..2 {
                let (sx, sy) = (2 * x + dx, 2 * y + dy);
                if sx < 1031 && sy < 517 {
                    let px = premultiplied_row_pixel(&p.planes().rows(sy).expect("row"), sx);
                    for c in 0..4 {
                        sum[c] += px[c] as u32;
                    }
                    n += 1;
                }
            }
        }
        let want: Vec<u8> = sum.iter().map(|s| average(*s, n)).collect();
        let got = stored_pixel(above, x, y);
        assert_eq!(got.to_vec(), want, "at {x},{y}");
    }
}

#[test]
fn a_crop_returns_that_levels_pixels_premultiplied() {
    let p = Built::new(900, 600, 5);
    let pyramid = ViewPyramid::rebuild(p.planes());
    let r = PsdRect {
        top: 20,
        left: 10,
        bottom: 36,
        right: 42,
    };

    let lvl0 = pyramid.crop(p.planes(), 0, r);
    assert_eq!((lvl0.width(), lvl0.height()), (32, 16));
    for y in 0..16usize {
        for x in 0..32usize {
            let want = premultiplied_row_pixel(&p.planes().rows(20 + y).expect("row"), 10 + x);
            let o = (y * 32 + x) * 4;
            assert_eq!(lvl0.data()[o..o + 4], want[..], "at {x},{y}");
        }
    }

    let lvl1 = pyramid.crop(p.planes(), 1, r);
    let stored = &pyramid.levels[0];
    for y in 0..16usize {
        for x in 0..32usize {
            let want = stored_pixel(stored, 10 + x, 20 + y);
            let o = (y * 32 + x) * 4;
            assert_eq!(lvl1.data()[o..o + 4], want[..], "at {x},{y}");
        }
    }
}

#[test]
fn a_crop_past_the_level_is_transparent() {
    let p = Built::new(300, 200, 1);
    let pyramid = ViewPyramid::rebuild(p.planes());
    let out = pyramid.crop(p.planes(), 0, rect(-5, -5, 10, 10));
    assert_eq!((out.width(), out.height()), (10, 10));
    for y in 0..5usize {
        for x in 0..5usize {
            let o = (y * 10 + x) * 4;
            assert_eq!(out.data()[o..o + 4], [0u8, 0, 0, 0][..]);
        }
    }
    // The overlapping 5x5 corner is opaque premultiplied level-0 pixels.
    let corner = premultiplied_row_pixel(&p.planes().rows(0).expect("row"), 0);
    let o = (5 * 10 + 5) * 4;
    assert_eq!(out.data()[o..o + 4], corner[..]);
}

#[test]
fn shrinking_premultiplied_keeps_a_soft_edge_its_colour() {
    let (w, h) = (600u32, 600u32);
    let n = w as usize * h as usize;
    let mut a = vec![255u8; n];
    let mut r = vec![0u8; n];
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = y * w as usize + x;
            if x % 2 == 1 {
                a[i] = 0;
            } else {
                r[i] = 255;
            }
        }
    }
    let p = Planes::new(w, h, &r, &[0u8; 600 * 600], &[0u8; 600 * 600], &a);
    let pyramid = ViewPyramid::rebuild(p);
    let out = pyramid.crop(p, 1, rect(10, 10, 1, 1));
    let o = 0;
    let alpha = out.data()[o + 3];
    assert!((alpha as i32 - 128).abs() <= 1, "alpha {alpha}");
    let unpremul_red = (out.data()[o] as u32 * 255 + alpha as u32 / 2) / alpha as u32;
    assert_eq!(unpremul_red, 255);
}

/// `p`'s four channels as one planar RGBA plane, tiled per channel, with
/// tile `fork` forked from a held clone so the plane is diverged (a
/// copy-on-write plane the pyramid must not materialize).
fn planar_plane(p: &Built, fork: usize) -> Plane<u8> {
    let mut data = Vec::with_capacity((p.width * p.height * 4) as usize);
    data.extend_from_slice(&p.r);
    data.extend_from_slice(&p.g);
    data.extend_from_slice(&p.b);
    data.extend_from_slice(&p.a);
    let mut plane = Plane::from(data);
    plane.tile(p.width as usize, p.height as usize);
    let mut copy = plane.clone();
    let (x, y, _, _) = copy.tile_rect(fork).expect("a tile to fork");
    drop(copy.tiles_mut(x..x + 1, y..y + 1, p.width as usize));
    assert!(!copy.shares(&plane), "the tile was forked");
    copy
}

#[test]
fn a_tiled_level0_builds_the_same_levels() {
    let p = Built::new(600, 400, 3);
    let plane = planar_plane(&p, 1);
    let banded = Planes::planar(p.width, p.height, &plane).expect("planar");

    let exact = ViewPyramid::rebuild(p.planes());
    let from_bands = ViewPyramid::rebuild(banded);
    assert_eq!(from_bands.level_count(), exact.level_count());
    for level in 1..exact.level_count() {
        assert!(
            from_bands.levels[level - 1].data() == exact.levels[level - 1].data(),
            "level {level} differs"
        );
    }

    assert!(
        !plane.is_mirrored(),
        "a tiled level 0 is never materialized"
    );
}

#[test]
fn a_tiled_level0_crops_without_materializing() {
    let source = Built::new(600, 401, 9);
    let plane = planar_plane(&source, 1);
    let tiled = Planes::planar(source.width, source.height, &plane).expect("planar");
    let reference = ViewPyramid::rebuild(source.planes());
    let from_tiles = ViewPyramid::rebuild(tiled);
    for level in 0..reference.level_count() {
        for crop in [rect(10, 20, 30, 40), rect(-5, -5, 13, 17)] {
            assert_eq!(
                from_tiles.crop(tiled, level, crop),
                reference.crop(source.planes(), level, crop)
            );
        }
    }
    assert!(
        !plane.is_mirrored(),
        "rebuild and crop read only source rows"
    );
}

fn rect(top: i32, left: i32, width: i32, height: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom: top + height,
        right: left + width,
    }
}
