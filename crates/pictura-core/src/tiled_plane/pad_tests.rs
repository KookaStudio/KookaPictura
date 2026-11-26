//! `Plane::padded` against a plain row copy, and the tiles it shares (#1609).

use crate::{Plane, TileState, TILE};

/// A `w × h × channels` plane with an empty first tile row, one uniform tile
/// and dense noise elsewhere, so every tile kind is padded.
fn source(w: usize, h: usize, channels: usize) -> Vec<u8> {
    let mut out = vec![0u8; w * h * channels];
    for c in 0..channels {
        for y in TILE..h {
            for x in 0..w {
                let v = if x < TILE && y < 2 * TILE {
                    7
                } else {
                    ((x * 31 + y * 17 + c * 5) % 251) as u8
                };
                out[(c * h + y) * w + x] = v;
            }
        }
    }
    out
}

fn tiled(data: &[u8], w: usize, h: usize) -> Plane<u8> {
    let mut plane: Plane<u8> = data.to_vec().into();
    assert!(plane.tile(w, h));
    plane
}

fn expected(
    data: &[u8],
    (w, h, channels): (usize, usize, usize),
    (nw, nh): (usize, usize),
    (x0, y0): (usize, usize),
) -> Vec<u8> {
    let mut out = vec![0u8; nw * nh * channels];
    for c in 0..channels {
        for y in 0..h {
            let from = (c * h + y) * w;
            let to = (c * nh + y0 + y) * nw + x0;
            out[to..to + w].copy_from_slice(&data[from..from + w]);
        }
    }
    out
}

#[test]
fn padded_matches_a_row_copy_at_every_offset() {
    let (w, h) = (600, 700);
    for channels in [1, 2] {
        let data = source(w, h, channels);
        let plane = tiled(&data, w, h);
        for (x0, y0) in [(0, 0), (TILE, 2 * TILE), (37, 0), (0, 101), (130, 259)] {
            let (nw, nh) = (w + x0 + 90, h + y0 + 300);
            let out = plane.padded(nw, nh, x0, y0).expect("a tiled plane pads");
            assert_eq!(out.geometry(), Some((nw, nh, channels)));
            assert!(
                out == expected(&data, (w, h, channels), (nw, nh), (x0, y0)),
                "channels {channels}, offset ({x0}, {y0})"
            );
        }
    }
}

#[test]
fn a_grow_right_and_down_shares_every_full_tile() {
    let (w, h) = (600, 700);
    let data = source(w, h, 1);
    let plane = tiled(&data, w, h);
    let held = plane.allocations();
    let out = plane.padded(w + 900, h + 900, 0, 0).expect("pads");
    let (cols, full_cols, full_rows) = (w.div_ceil(TILE), w / TILE, h / TILE);
    let new_cols = (w + 900).div_ceil(TILE);
    for ty in 0..h.div_ceil(TILE) {
        for tx in 0..cols {
            let (old, new) = (ty * cols + tx, ty * new_cols + tx);
            if tx < full_cols && ty < full_rows {
                assert_eq!(
                    out.tile_state(new),
                    plane.tile_state(old),
                    "tile ({tx}, {ty})"
                );
            }
        }
    }
    let old_full_dense = (0..plane.tile_count())
        .filter(|&i| {
            let (x, y, tw, th) = plane.tile_rect(i).expect("a tile");
            (tw, th) == (TILE, TILE) && x + TILE <= w && y + TILE <= h
        })
        .filter(|&i| matches!(plane.tile_state(i), Some(TileState::Dense(_))))
        .count();
    let shared = out
        .allocations()
        .iter()
        .filter(|a| held.contains(a))
        .count();
    assert_eq!(shared, old_full_dense, "every full dense tile is shared");
    assert!(out == expected(&data, (w, h, 1), (w + 900, h + 900), (0, 0)));
}

#[test]
fn empty_tiles_are_not_copied() {
    let (w, h) = (4 * TILE, 4 * TILE);
    let mut data = vec![0u8; w * h];
    data[TILE * w + TILE + 3] = 200;
    let plane = tiled(&data, w, h);
    let out =
        crate::assert_no_materialization(|| plane.padded(w + 300, h + 300, 77, 141).expect("pads"));
    let dense = (0..out.tile_count())
        .filter(|&i| matches!(out.tile_state(i), Some(TileState::Dense(_))))
        .count();
    assert_eq!(dense, 1, "only the one painted tile is written");
    assert!(out == expected(&data, (w, h, 1), (w + 300, h + 300), (77, 141)));
}

#[test]
fn a_flat_plane_or_a_window_too_small_is_refused() {
    let flat: Plane<u8> = vec![1u8; 16].into();
    assert!(flat.padded(8, 8, 0, 0).is_none());
    let plane = tiled(&[1u8; 16], 4, 4);
    assert!(plane.padded(4, 4, 1, 0).is_none());
}
