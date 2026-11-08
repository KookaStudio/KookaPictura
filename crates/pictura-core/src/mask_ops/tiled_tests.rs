//! The tiled flood and the banded kernels against the dense ones (#1214).

use super::{
    coverage_bounds, dilate, erode, flood, flood_tiled, map_bands, match_mask, median, Border,
    FloodSource, Grid, MatchTiles, Seeds, TileMatch,
};
use crate::tiled_plane::{TileState, TILE};
use crate::Plane;

fn next(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// A `w × h` image of random blocks: each `block²` square holds one value
/// drawn from `levels`, so some tiles are uniform and the flood has walls.
fn blocks(w: usize, h: usize, block: usize, levels: &[u8], seed: u64) -> Vec<u8> {
    let mut state = seed | 1;
    let (bw, bh) = (w.div_ceil(block), h.div_ceil(block));
    let cells: Vec<u8> = (0..bw * bh)
        .map(|_| levels[(next(&mut state) % levels.len() as u64) as usize])
        .collect();
    (0..w * h)
        .map(|i| cells[(i / w / block) * bw + (i % w) / block])
        .collect()
}

/// Per-pixel noise over a block image, on a quarter of the blocks.
fn speckle(image: &mut [u8], w: usize, seed: u64) {
    let mut state = seed | 1;
    for (i, px) in image.iter_mut().enumerate() {
        let (x, y) = (i % w, i / w);
        if (x / 97 + y / 61) % 4 == 0 {
            *px = (next(&mut state) & 0xff) as u8;
        }
    }
}

fn tiled(data: &[u8], w: usize, h: usize) -> Plane<u8> {
    let mut plane = Plane::from(data.to_vec());
    assert!(plane.tile(w, h));
    plane
}

/// The tile `t` of a dense image is one value.
fn uniform_tile(data: &[u8], grid: &Grid, t: usize) -> Option<u8> {
    let (x0, y0, tw, th) = grid.rect(t);
    let first = data[y0 * grid.w + x0];
    (y0..y0 + th)
        .all(|y| {
            data[y * grid.w + x0..y * grid.w + x0 + tw]
                .iter()
                .all(|&v| v == first)
        })
        .then_some(first)
}

/// The bucket-style predicate as a per-tile source: a pixel matches when it
/// is within `tol` of `target` and its wall is open.
fn eval_tile<'a>(
    image: &'a [u8],
    wall: Option<&'a [u8]>,
    grid: Grid,
    target: u8,
    tol: u8,
) -> impl Fn(usize, (usize, usize, usize, usize)) -> TileMatch + Sync + 'a {
    move |t, (x0, y0, tw, th)| {
        let src = uniform_tile(image, &grid, t);
        let open = wall.map_or(Some(255), |wall| uniform_tile(wall, &grid, t));
        match (src, open) {
            (_, Some(0)) => return TileMatch::None,
            (Some(v), Some(_)) => {
                return if v.abs_diff(target) <= tol {
                    TileMatch::All
                } else {
                    TileMatch::None
                };
            }
            _ => {}
        }
        TileMatch::from_rows(tw, th, |dy, out| {
            for (dx, o) in out.iter_mut().enumerate() {
                let i = (y0 + dy) * grid.w + x0 + dx;
                *o = wall.is_none_or(|wall| wall[i] > 0) && image[i].abs_diff(target) <= tol;
            }
        })
    }
}

fn dense_flood(image: &[u8], wall: Option<&[u8]>, w: usize, seed: usize, tol: u8) -> Vec<u8> {
    let target = image[seed];
    let open = |i: usize| wall.is_none_or(|wall| wall[i] > 0);
    flood(w, image.len() / w, &one_seed(image.len(), seed), |_, to| {
        open(to) && image[to].abs_diff(target) <= tol
    })
    .unwrap_or_else(|| vec![0; image.len()])
}

fn one_seed(n: usize, at: usize) -> Vec<u8> {
    let mut seeds = vec![0u8; n];
    seeds[at] = 1;
    seeds
}

#[test]
fn tiled_flood_matches_dense() {
    for (k, &(w, h)) in [(300usize, 700usize), (513, 257), (600, 600)]
        .iter()
        .enumerate()
    {
        let mut image = blocks(w, h, 37, &[10, 14, 60, 200, 250], 7 + k as u64);
        speckle(&mut image, w, 11 + k as u64);
        let wall = blocks(w, h, 131, &[0, 255, 255, 128], 23 + k as u64);
        let grid = Grid::new(w, h);
        for wall in [None, Some(wall.as_slice())] {
            for tol in [0u8, 8, 40, 255] {
                for seed in [0usize, w * h / 2 + w / 3, w * h - 1] {
                    if wall.is_some_and(|wall| wall[seed] == 0) {
                        continue;
                    }
                    let target = image[seed];
                    let mut source =
                        MatchTiles::new(w, h, eval_tile(&image, wall, grid, target, tol));
                    let mask = flood_tiled(w, h, Seeds::Pixel(seed % w, seed / w), &mut source)
                        .expect("the seed is in the plane");
                    let want = dense_flood(&image, wall, w, seed, tol);
                    assert!(
                        mask == want,
                        "{w}x{h} tol {tol} seed {seed} wall {}",
                        wall.is_some()
                    );
                    let global = match_mask(w, h, eval_tile(&image, wall, grid, target, tol));
                    let want: Vec<u8> = (0..w * h)
                        .map(|i| {
                            let open = wall.is_none_or(|wall| wall[i] > 0);
                            u8::from(open && image[i].abs_diff(target) <= tol) * 255
                        })
                        .collect();
                    assert!(global == want, "global {w}x{h} tol {tol}");
                }
            }
        }
    }
}

/// A pixel-pair predicate (Grow's) with a seed mask: the reached set is the
/// same whatever order the tiles are visited in.
#[test]
fn tiled_flood_from_a_seed_mask_matches_dense() {
    struct Pairs<'a> {
        image: &'a [u8],
        grid: Grid,
        tol: u8,
    }
    impl FloodSource for Pairs<'_> {
        fn matches(&mut self, (fx, fy): (usize, usize), (tx, ty): (usize, usize)) -> bool {
            let w = self.grid.w;
            self.image[fy * w + fx].abs_diff(self.image[ty * w + tx]) <= self.tol
        }
        fn whole(&mut self, tile: usize) -> bool {
            uniform_tile(self.image, &self.grid, tile).is_some()
        }
    }
    let (w, h) = (700, 530);
    let mut image = blocks(w, h, 41, &[0, 5, 90, 100, 255], 3);
    speckle(&mut image, w, 5);
    let seeds = blocks(w, h, 300, &[0, 0, 0, 180], 9);
    for tol in [0u8, 6, 30] {
        let want = flood(w, h, &seeds, |from, to| {
            image[from].abs_diff(image[to]) <= tol
        })
        .unwrap_or_else(|| vec![0; w * h]);
        let mut source = Pairs {
            image: &image,
            grid: Grid::new(w, h),
            tol,
        };
        let seed_mask = tiled(&seeds, w, h);
        let got = flood_tiled(w, h, Seeds::Mask(&seed_mask), &mut source)
            .unwrap_or_else(|| super::empty_mask(w, h));
        assert!(got == want, "tol {tol}");
        let flat = Plane::from(seeds.clone());
        let got = flood_tiled(w, h, Seeds::Mask(&flat), &mut source)
            .unwrap_or_else(|| super::empty_mask(w, h));
        assert!(got == want, "flat seeds, tol {tol}");
    }
}

#[test]
fn visited_state_covers_only_reached_tiles() {
    let (w, h) = (2048, 2048);
    // A closed 50² room of value 9 inside tile (2, 3), walled by value 200.
    let mut image = vec![200u8; w * h];
    for y in 3 * TILE + 20..3 * TILE + 70 {
        for x in 2 * TILE + 30..2 * TILE + 80 {
            image[y * w + x] = 9;
        }
    }
    let grid = Grid::new(w, h);
    let mut source = MatchTiles::new(w, h, eval_tile(&image, None, grid, 9, 0));
    let seed = (2 * TILE + 40, 3 * TILE + 40);
    let (mask, touched) =
        super::flood_tiled::flood_tiled_touched(w, h, Seeds::Pixel(seed.0, seed.1), &mut source)
            .expect("the seed is in the plane");
    assert_eq!(touched, 1, "visited state only for the room's tile");
    let room = 3 * grid.cols + 2;
    for t in 0..grid.count() {
        let state = mask.tile_state(t).expect("tiled");
        if t == room {
            assert!(
                matches!(state, TileState::Dense(_)),
                "the room's tile is dense"
            );
        } else {
            assert_eq!(state, TileState::Empty, "tile {t} is not reached");
        }
    }
    assert_eq!(mask.iter().filter(|&&v| v == 255).count(), 50 * 50);
}

#[test]
fn a_uniform_source_floods_whole_tiles() {
    let (w, h) = (4250, 4250);
    let grid = Grid::new(w, h);
    let mut source = MatchTiles::new(w, h, |_: usize, _: (usize, usize, usize, usize)| {
        TileMatch::All
    });
    let mask = flood_tiled(w, h, Seeds::Pixel(w / 2, h / 2), &mut source)
        .expect("the seed is in the plane");
    for t in 0..grid.count() {
        assert_eq!(
            mask.tile_state(t),
            Some(TileState::Uniform(255)),
            "tile {t}"
        );
    }
    assert_eq!(mask.allocated_bytes(), 0);
}

/// The dense kernel `f` and its banded run agree byte for byte, on a tiled
/// and on a flat mask.
fn banded_matches(
    data: &[u8],
    w: usize,
    h: usize,
    pad: usize,
    f: impl Fn(&[u8], usize, usize) -> Vec<u8> + Sync,
    what: &str,
) {
    let want = f(data, w, h);
    let got = map_bands(&tiled(data, w, h), w, h, pad, &f);
    assert!(got.is_tiled());
    assert!(got == want, "{what}: tiled input");
    let got = map_bands(&Plane::from(data.to_vec()), w, h, pad, &f);
    assert!(got == want, "{what}: flat input");
}

#[test]
fn banded_kernels_match_the_whole_mask() {
    let (w, h) = (700, 590);
    let mut data = blocks(w, h, 90, &[0, 0, 255, 255, 128], 17);
    speckle(&mut data, w, 19);
    for r in [1usize, 4, 40] {
        banded_matches(
            &data,
            w,
            h,
            r + 1,
            |s, w, h| dilate(s, w, h, r, Border::Zero(0)),
            "dilate",
        );
        banded_matches(
            &data,
            w,
            h,
            r + 1,
            |s, w, h| erode(s, w, h, r, Border::Zero(0)),
            "erode",
        );
        banded_matches(
            &data,
            w,
            h,
            r + 1,
            |s, w, h| dilate(s, w, h, r, Border::Clamp),
            "clamp",
        );
        banded_matches(&data, w, h, r + 1, |s, w, h| median(s, w, h, r), "median");
    }
}

#[test]
fn coverage_bounds_reads_tiles() {
    let (w, h) = (900, 700);
    let mut data = vec![0u8; w * h];
    for y in 300..650 {
        for x in 260..700 {
            data[y * w + x] = 255;
        }
    }
    data[299 * w + 811] = 3;
    let want = Some((260, 299, 812, 650));
    assert_eq!(coverage_bounds(&tiled(&data, w, h), w, h), want);
    assert_eq!(coverage_bounds(&Plane::from(data.clone()), w, h), want);
    assert_eq!(coverage_bounds(&super::empty_mask(w, h), w, h), None);
}
