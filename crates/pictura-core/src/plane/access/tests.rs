use super::super::{materializations, Plane};
use super::assert_no_materialization;
use crate::tiled_plane::TILE;

const WIDTH: usize = 300;
const HEIGHT: usize = 2 * TILE + 5;

fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i.wrapping_mul(37) % 251) as u8).collect()
}

/// A tiled plane whose middle-row tile 2 (`x 0..256`, `y 256..512`) has
/// diverged from a live clone, plus the logical bytes it holds.
fn diverged() -> (Plane<u8>, Plane<u8>, Vec<u8>) {
    let mut plane: Plane<u8> = pattern(WIDTH * HEIGHT).into();
    plane.tile(WIDTH, HEIGHT);
    let sharer = plane.clone();
    for tile in plane.tiles_mut(0..1, TILE..TILE + 1, WIDTH) {
        tile.data.iter_mut().for_each(|b| *b = b.wrapping_add(3));
    }
    plane.settle();
    assert!(!plane.shares_tile(2, &sharer));
    let mut expected = pattern(WIDTH * HEIGHT);
    for y in TILE..2 * TILE {
        for b in &mut expected[y * WIDTH..y * WIDTH + TILE] {
            *b = b.wrapping_add(3);
        }
    }
    (plane, sharer, expected)
}

fn rect(bytes: &[u8], xs: std::ops::Range<usize>, ys: std::ops::Range<usize>) -> Vec<u8> {
    ys.flat_map(|y| bytes[y * WIDTH + xs.start..y * WIDTH + xs.end].to_vec())
        .collect()
}

#[test]
fn rows_copy_out_of_a_diverged_plane_across_tile_edges() {
    let (plane, _sharer, expected) = diverged();
    let ys = TILE - 2..2 * TILE + 3;
    let rows: Vec<Vec<u8>> = assert_no_materialization(|| {
        ys.clone()
            .map(|y| {
                let mut row = vec![0; WIDTH];
                assert!(plane.read_row_into(y, WIDTH, &mut row));
                row
            })
            .collect()
    });
    let want: Vec<Vec<u8>> = expected
        .chunks(WIDTH)
        .skip(ys.start)
        .take(ys.len())
        .map(<[u8]>::to_vec)
        .collect();
    assert_eq!(rows, want);
    assert!(!plane.is_mirrored());

    let mut past = vec![0xAA; WIDTH];
    assert!(!plane.read_row_into(HEIGHT, WIDTH, &mut past));
    assert!(
        past.iter().all(|&b| b == 0xAA),
        "a row past the end is untouched"
    );
}

#[test]
fn read_rect_into_matches_the_slice_and_clips_to_the_plane() {
    let (plane, _sharer, expected) = diverged();
    let (xs, ys) = (250..262, TILE - 1..2 * TILE + 1);
    let mut out = vec![0; xs.len() * ys.len()];
    assert_no_materialization(|| plane.read_rect_into(xs.clone(), ys.clone(), WIDTH, &mut out));
    assert_eq!(out, rect(&expected, xs, ys));

    let (xs, ys) = (WIDTH - 2..WIDTH + 3, HEIGHT - 1..HEIGHT + 1);
    let mut out = vec![0xAA; xs.len() * ys.len()];
    plane.read_rect_into(xs.clone(), ys, WIDTH, &mut out);
    let last = &expected[(HEIGHT - 1) * WIDTH..];
    assert_eq!(&out[..2], &last[WIDTH - 2..]);
    assert!(out[2..].iter().all(|&b| b == 0xAA));
}

#[test]
fn read_rect_into_an_empty_rect_copies_nothing() {
    let (plane, _sharer, _) = diverged();
    let mut out = vec![9u8; 4];
    plane.read_rect_into(3..3, 0..4, WIDTH, &mut out);
    plane.read_rect_into(0..4, 5..5, WIDTH, &mut out);
    plane.read_rect_into(0..4, HEIGHT..HEIGHT + 1, WIDTH, &mut out);
    assert_eq!(out, vec![9; 4]);
}

#[test]
fn read_into_copies_a_logical_range_across_tiles() {
    let (plane, _sharer, expected) = diverged();
    let start = TILE * WIDTH - 3;
    let mut out = vec![0; WIDTH * 2 + 6];
    assert_no_materialization(|| plane.read_into(start, &mut out));
    assert_eq!(out, expected[start..start + out.len()]);

    let mut tail = vec![0xAA; 5];
    plane.read_into(expected.len() - 2, &mut tail);
    assert_eq!(&tail[..2], &expected[expected.len() - 2..]);
    assert_eq!(&tail[2..], &[0xAA; 3], "past the end stays untouched");
    let mut none = vec![0xAA; 2];
    plane.read_into(expected.len() + 4, &mut none);
    assert_eq!(none, vec![0xAA; 2]);
}

#[test]
fn for_each_tile_visits_every_tile_with_its_position() {
    let (plane, _sharer, expected) = diverged();
    let mut seen = vec![0u8; expected.len()];
    let mut rects = Vec::new();
    assert_no_materialization(|| {
        plane.for_each_tile(0..WIDTH, 0..HEIGHT, WIDTH, |tile| {
            rects.push((tile.x, tile.y, tile.w, tile.h));
            for dy in 0..tile.h {
                let at = (tile.y + dy) * WIDTH + tile.x;
                tile.copy_row(dy, 0..tile.w, &mut seen[at..at + tile.w]);
            }
        })
    });
    assert_eq!(rects.len(), 6);
    assert_eq!(rects[3], (256, 256, 44, 256));
    assert_eq!(rects[5], (256, 512, 44, 5));
    assert_eq!(seen, expected);
}

#[test]
fn a_flat_plane_presents_full_width_strips() {
    let bytes = pattern(7 * 300);
    let plane: Plane<u8> = bytes.clone().into();
    let mut strips = Vec::new();
    plane.for_each_tile(0..7, 10..290, 7, |tile| {
        strips.push((tile.y, tile.h, tile.w));
        assert_eq!(tile.row(0), Some(&bytes[tile.y * 7..tile.y * 7 + 7]));
    });
    assert_eq!(strips, vec![(0, 256, 7), (256, 44, 7)]);
}

#[test]
fn a_mutable_access_forks_only_the_tiles_it_touches() {
    let (mut plane, sharer, mut expected) = diverged();
    let before = sharer.as_slice().to_vec();
    let y = 2 * TILE + 1;
    assert_no_materialization(|| {
        plane.write_row_from(y, WIDTH, &[0x11; WIDTH]);
    });
    expected[y * WIDTH..(y + 1) * WIDTH].fill(0x11);
    assert_eq!(plane, expected);
    assert!(
        plane.shares_tile(0, &sharer),
        "an untouched tile stays shared"
    );
    assert!(!plane.shares_tile(4, &sharer), "the written tile forked");
    assert_eq!(sharer, before, "the sharer keeps its bytes");

    assert_no_materialization(|| {
        plane.for_each_tile_mut(260..261, 0..1, WIDTH, |mut tile| {
            tile.row_mut(0)[4] = 0x22;
        })
    });
    expected[260] = 0x22;
    assert_eq!(plane, expected);
    assert!(!plane.shares_tile(1, &sharer));
    assert!(plane.shares_tile(0, &sharer));
}

#[test]
fn write_rect_from_round_trips_and_clips() {
    let (mut plane, sharer, mut expected) = diverged();
    let (xs, ys) = (1..5, TILE - 2..TILE + 2);
    let src: Vec<u8> = (0..(xs.len() * ys.len()) as u8).collect();
    assert_no_materialization(|| plane.write_rect_from(xs.clone(), ys.clone(), WIDTH, &src));
    for (dy, y) in ys.clone().enumerate() {
        let at = y * WIDTH;
        expected[at + xs.start..at + xs.end].copy_from_slice(&src[dy * 4..dy * 4 + 4]);
    }
    assert_eq!(plane, expected);
    let mut back = vec![0; src.len()];
    plane.read_rect_into(xs, ys, WIDTH, &mut back);
    assert_eq!(back, src);
    assert!(plane.shares_tile(5, &sharer));

    plane.write_rect_from(
        WIDTH - 1..WIDTH + 1,
        HEIGHT - 1..HEIGHT + 1,
        WIDTH,
        &[7, 8, 9, 10],
    );
    expected[HEIGHT * WIDTH - 1] = 7;
    assert_eq!(plane, expected, "only the in-plane sample is written");
}

#[test]
fn flat_planes_serve_the_same_access() {
    let mut flat: Plane<u8> = pattern(7 * 3).into();
    let mut row = [0u8; 7];
    assert!(flat.read_row_into(2, 7, &mut row));
    assert_eq!(&row[..], &pattern(21)[14..]);
    assert!(!flat.read_row_into(3, 7, &mut row));
    flat.write_rect_from(5..9, 2..4, 7, &[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(&flat[19..21], &[1, 2]);
    for mut tile in flat.tiles_mut(0..7, 0..3, 7) {
        tile.row_mut(0)[0] = 42;
    }
    assert_eq!(flat[0], 42);
    assert!(!flat.is_tiled());
}

#[test]
fn assert_no_materialization_catches_a_whole_plane_copy() {
    let (plane, _sharer, _) = diverged();
    let start = materializations();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert_no_materialization(|| plane.as_slice().len())
    }));
    assert!(materializations() > start);
    // Outside nextest the guard runs unchecked; see its doc.
    let alone = std::env::var_os("NEXTEST").is_some();
    assert_eq!(
        caught.is_err(),
        alone,
        "the guard panics exactly under nextest"
    );
}

#[test]
fn a_span_inside_one_dense_tile_is_copied() {
    // A tile's samples may be evicted once its pin drops (#1217), so a span
    // is copied rather than borrowed past the call.
    let (plane, _sharer, expected) = diverged();
    let y = TILE + 3;
    let span = plane.row_span(y, 10..40, WIDTH).expect("a span");
    assert!(matches!(span, std::borrow::Cow::Owned(_)));
    assert_eq!(&span[..], &expected[y * WIDTH + 10..y * WIDTH + 40]);
    let across = plane
        .row_span(y, 250..262, WIDTH)
        .expect("a span across tiles");
    assert_eq!(&across[..], &expected[y * WIDTH + 250..y * WIDTH + 262]);
    assert!(
        plane.row_span(y, 290..WIDTH + 1, WIDTH).is_none(),
        "past the row"
    );
}
