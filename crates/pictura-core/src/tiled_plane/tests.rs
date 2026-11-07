use super::{TileSample, TileState, TILE};
use crate::{fresh_stamp, Plane};

/// A fixed-seed xorshift, so every run walks the same operations (rule 6).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

trait Gen: TileSample + std::fmt::Debug {
    fn from_seed(v: u64) -> Self;
}

impl Gen for u8 {
    fn from_seed(v: u64) -> Self {
        // A small alphabet, so uniform tiles and collapses actually happen.
        [0, 7, 200, 255][(v % 4) as usize]
    }
}

impl Gen for u16 {
    fn from_seed(v: u64) -> Self {
        [0, 7, 40000, 65535][(v % 4) as usize]
    }
}

impl Gen for f32 {
    fn from_seed(v: u64) -> Self {
        [0.0, -0.0, 0.5, 1.0][(v % 4) as usize]
    }
}

fn bits_eq<T: TileSample>(a: &[T], b: &[T]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.same(*y))
}

/// Run 10 000 seeded operations against a tiled plane and a `Vec`, checking
/// every read.
fn run_oracle<T: Gen>(width: usize, height: usize, channels: usize, seed: u64) {
    let mut rng = Rng(seed);
    let rows = height * channels;
    let mut reference: Vec<T> = (0..width * rows)
        .map(|_| T::from_seed(rng.next()))
        .collect();
    let mut plane: Plane<T> = reference.clone().into();
    assert!(plane.tile(width, height));
    let mut held: Option<(Plane<T>, Vec<T>)> = None;
    for step in 0..10_000 {
        let x0 = rng.below(width);
        let y0 = rng.below(rows);
        let w = 1 + rng.below((width - x0).min(300));
        let h = 1 + rng.below((rows - y0).min(300));
        match rng.below(9) {
            0 => {
                let v = T::from_seed(rng.next());
                let src = vec![v; w * h];
                plane.write_rect_from(x0..x0 + w, y0..y0 + h, width, &src);
                for y in y0..y0 + h {
                    reference[y * width + x0..y * width + x0 + w].fill(v);
                }
            }
            1 => {
                let src: Vec<T> = (0..w * h).map(|_| T::from_seed(rng.next())).collect();
                plane.write_rect_from(x0..x0 + w, y0..y0 + h, width, &src);
                for (dy, y) in (y0..y0 + h).enumerate() {
                    reference[y * width + x0..y * width + x0 + w]
                        .copy_from_slice(&src[dy * w..dy * w + w]);
                }
            }
            2 => {
                let src: Vec<T> = (0..width).map(|_| T::from_seed(rng.next())).collect();
                plane.write_row_from(y0, width, &src);
                reference[y0 * width..y0 * width + width].copy_from_slice(&src);
            }
            3 => {
                let v = T::from_seed(rng.next());
                let xs = x0..x0 + w;
                let ys = y0..y0 + h;
                for mut tile in plane.tiles_mut(xs.clone(), ys.clone(), width) {
                    let (tx, ty) = (tile.x, tile.y);
                    for dy in 0..tile.h {
                        if !ys.contains(&(ty + dy)) {
                            continue;
                        }
                        let row = tile.row_mut(dy);
                        for (dx, s) in row.iter_mut().enumerate() {
                            if xs.contains(&(tx + dx)) {
                                *s = v;
                            }
                        }
                    }
                }
                for y in ys {
                    reference[y * width + xs.start..y * width + xs.end].fill(v);
                }
            }
            4 => plane.settle(),
            5 => {
                let mut out = vec![T::default(); w * h];
                plane.read_rect_into(x0..x0 + w, y0..y0 + h, width, &mut out);
                for (dy, y) in (y0..y0 + h).enumerate() {
                    let want = &reference[y * width + x0..y * width + x0 + w];
                    assert!(
                        bits_eq(&out[dy * w..dy * w + w], want),
                        "rect at step {step}"
                    );
                }
            }
            6 => {
                let mut out = vec![T::default(); width];
                assert!(plane.read_row_into(y0, width, &mut out));
                assert!(bits_eq(&out, &reference[y0 * width..y0 * width + width]));
                let i = y0 * width + x0;
                assert!(plane.sample(i).unwrap().same(reference[i]), "sample {i}");
                let n = rng.below(reference.len() - i) + 1;
                let mut span = vec![T::default(); n];
                plane.read_into(i, &mut span);
                assert!(bits_eq(&span, &reference[i..i + n]), "span at step {step}");
            }
            7 => match held.take() {
                Some((copy, bytes)) => {
                    let mut out = vec![T::default(); bytes.len()];
                    copy.read_into(0, &mut out);
                    assert!(bits_eq(&out, &bytes), "a held clone kept its bytes");
                }
                None => held = Some((plane.clone(), reference.clone())),
            },
            _ => {
                let mut out = vec![T::default(); reference.len()];
                plane.read_into(0, &mut out);
                assert!(bits_eq(&out, &reference), "whole plane at step {step}");
            }
        }
    }
    plane.settle();
    assert!(
        bits_eq(plane.as_slice(), &reference),
        "the materialized view"
    );
}

#[test]
fn matches_vec_reference() {
    let sizes = [
        (1, 1, 1),
        (255, 257, 1),
        (256, 256, 1),
        (300, 700, 1),
        (300, 300, 3),
    ];
    for (k, &(w, h, c)) in sizes.iter().enumerate() {
        let seed = 0x9E37_79B9_7F4A_7C15 ^ k as u64;
        run_oracle::<u8>(w, h, c, seed);
        run_oracle::<u16>(w, h, c, seed);
        run_oracle::<f32>(w, h, c, seed);
    }
}

fn varied(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i.wrapping_mul(37) % 251) as u8).collect()
}

#[test]
fn one_pixel_write_forks_one_tile() {
    let side = 1024;
    let mut original: Plane<u8> = varied(side * side).into();
    original.tile(side, side);
    let mut copy = original.clone();
    copy.write_rect_from(300..301, 700..701, side, &[1]);
    let forked: Vec<usize> = (0..copy.tile_count())
        .filter(|&i| !copy.shares_tile(i, &original))
        .collect();
    let written = 2 * (side / TILE) + 1;
    assert_eq!(forked, vec![written], "only the written tile forked");
    assert_eq!(
        copy.tile_state(written),
        Some(TileState::Dense(TILE * TILE))
    );
    let fresh: Vec<_> = copy
        .allocations()
        .into_iter()
        .filter(|a| !original.allocations().contains(a))
        .collect();
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].1, 65_536, "one 64 KiB tile");
    assert_eq!(
        original.sample(700 * side + 300),
        Some(varied(side * side)[700 * side + 300])
    );
    assert_eq!(copy.sample(700 * side + 300), Some(1));
}

#[test]
fn tiles_restart_at_each_channel() {
    let (w, h) = (300, 300);
    let mut plane: Plane<u8> = varied(w * h * 3).into();
    plane.tile(w, h);
    assert_eq!(plane.tile_count(), 3 * 4, "2 × 2 tiles per channel");
    assert_eq!(
        plane.tile_rect(4),
        Some((0, 300, 256, 256)),
        "channel 1 starts a grid"
    );
    let before = plane.clone();
    plane.write_row_from(h - 1, w, &vec![9; w]);
    for i in 4..12 {
        assert!(plane.shares_tile(i, &before), "tile {i} of a later channel");
    }
}

#[test]
fn uniform_collapse_per_channel() {
    let (w, h) = (300, 260);
    let mut data = vec![7u8; w * h * 3];
    data[w * h + 5] = 8;
    data[2 * w * h..].fill(0);
    let mut plane: Plane<u8> = data.clone().into();
    plane.tile(w, h);
    for i in 0..4 {
        assert_eq!(
            plane.tile_state(i),
            Some(TileState::Uniform(7)),
            "channel 0"
        );
    }
    assert!(
        matches!(plane.tile_state(4), Some(TileState::Dense(_))),
        "the odd sample"
    );
    for i in 5..8 {
        assert_eq!(plane.tile_state(i), Some(TileState::Uniform(7)));
    }
    for i in 8..12 {
        assert_eq!(plane.tile_state(i), Some(TileState::Empty), "channel 2");
    }
    assert_eq!(plane, data);

    plane.write_rect_from(5..6, h..h + 1, w, &[7]);
    plane.settle();
    assert_eq!(
        plane.tile_state(4),
        Some(TileState::Uniform(7)),
        "a settled write collapses"
    );
}

#[test]
fn default_value_tiles_are_empty() {
    let side = 17_000;
    let white: Plane<u8> = Plane::filled(side, side, 1, 255);
    assert_eq!(white.allocated_bytes(), 0);
    assert_eq!(white.sample(side * side - 1), Some(255));
    let black: Plane<u8> = Plane::filled(side, side, 1, 0);
    assert_eq!(black.tile_state(0), Some(TileState::Empty));
    assert_eq!(black.allocated_bytes(), 0);

    let mut zeros: Plane<u8> = vec![0; 600 * 300].into();
    zeros.tile(600, 300);
    assert_eq!(zeros.allocated_bytes(), 0, "zeros tile into empty tiles");
    assert!(zeros.all_eq(0));
}

#[test]
fn stamp_and_ident_cleared_by_tile_writes() {
    let side = 600;
    let mut original: Plane<u8> = varied(side * side).into();
    original.tile(side, side);
    let stamp = fresh_stamp();
    original.set_stamp(stamp);
    let ident = original.ident();
    let writes: [fn(&mut Plane<u8>); 4] = [
        |p| drop(p.tiles_mut(0..1, 0..1, 600)),
        |p| p.write_rect_from(0..2, 0..2, 600, &[1, 2, 3, 4]),
        |p| {
            p.write_row_from(3, 600, &[5; 600]);
        },
        |p| {
            let other = p.clone();
            p.share_tile_from(0, &other);
        },
    ];
    for write in writes {
        let mut copy = original.clone();
        assert_eq!((copy.stamp(), copy.ident()), (stamp, ident));
        write(&mut copy);
        assert_eq!(copy.stamp(), 0, "a write clears the stamp");
        assert_ne!(copy.ident(), ident, "a write retires the ident");
        assert!(
            copy.shares_tile(8, &original),
            "an unwritten tile keeps its generation"
        );
    }
    assert_eq!((original.stamp(), original.ident()), (stamp, ident));
}

#[test]
fn tile_generation_survives_clone_and_settle() {
    let mut plane: Plane<u8> = Plane::filled(512, 512, 1, 3);
    plane.write_rect_from(0..2, 0..1, 512, &[1, 2]);
    let gen = plane.tile_generation(0);
    assert_eq!(plane.clone().tile_generation(0), gen);
    plane.set_stamp(42);
    let ident = plane.ident();
    for tile in plane.tiles_mut(0..1, 0..1, 512) {
        tile.data.fill(9);
    }
    let gen = plane.tile_generation(0);
    plane.set_stamp(43);
    let ident_after = plane.ident();
    assert_ne!(ident_after, ident);
    plane.settle();
    assert_eq!(plane.tile_state(0), Some(TileState::Uniform(9)));
    assert_eq!(
        plane.tile_generation(0),
        gen,
        "a collapse keeps the generation"
    );
    assert_eq!((plane.stamp(), plane.ident()), (43, ident_after));
}

#[test]
fn a_write_of_the_uniform_value_forks_nothing() {
    let mut plane: Plane<u8> = Plane::filled(1000, 1000, 1, 255);
    let before = plane.clone();
    plane.write_row_from(10, 1000, &[255; 1000]);
    plane.write_rect_from(0..300, 0..300, 1000, &vec![255; 300 * 300]);
    assert_eq!(plane.allocated_bytes(), 0);
    assert!((0..plane.tile_count()).all(|i| plane.shares_tile(i, &before)));
}

#[test]
fn build_tiled_fills_strips() {
    let (w, h) = (300, 600);
    let plane: Plane<u8> = Plane::build_tiled(w, h, 2, |y, row| {
        row.fill(if y < h { 255 } else { (y % 7) as u8 });
        Ok::<(), ()>(())
    })
    .unwrap();
    assert_eq!(plane.geometry(), Some((w, h, 2)));
    assert_eq!(plane.tile_state(0), Some(TileState::Uniform(255)));
    assert_eq!(plane.sample(h * w + 3 * w), Some(((h + 3) % 7) as u8));
    let failed: Result<Plane<u8>, &str> =
        Plane::build_tiled(
            w,
            h,
            1,
            |y, _| {
                if y == 400 {
                    Err("short read")
                } else {
                    Ok(())
                }
            },
        );
    assert_eq!(failed.err(), Some("short read"));
}
