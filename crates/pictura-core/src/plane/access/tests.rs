use super::super::{materializations, Plane, BAND_ROWS};
use super::assert_no_materialization;

const WIDTH: usize = 7;
const HEIGHT: usize = BAND_ROWS * 3 - 5;

fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i.wrapping_mul(37) % 251) as u8).collect()
}

/// A fractured plane whose band 1 has diverged from a live clone, plus the
/// logical bytes it holds (read from a separate clone's `as_slice`).
fn diverged() -> (Plane<u8>, Plane<u8>, Vec<u8>) {
    let mut plane: Plane<u8> = pattern(WIDTH * HEIGHT).into();
    plane.fracture(WIDTH);
    let sharer = plane.clone();
    for byte in plane.write_band(1).expect("band 1").iter_mut() {
        *byte = byte.wrapping_add(3);
    }
    assert!(plane.diverged());
    let expected = plane.clone().as_slice().to_vec();
    (plane, sharer, expected)
}

fn rect(bytes: &[u8], xs: std::ops::Range<usize>, ys: std::ops::Range<usize>) -> Vec<u8> {
    ys.flat_map(|y| bytes[y * WIDTH + xs.start..y * WIDTH + xs.end].to_vec())
        .collect()
}

#[test]
fn rows_match_the_materialized_slice_across_band_edges() {
    let (plane, _sharer, expected) = diverged();
    let ys = BAND_ROWS - 2..2 * BAND_ROWS + 3;
    let rows: Vec<Vec<u8>> =
        assert_no_materialization(|| plane.rows(ys.clone(), WIDTH).map(<[u8]>::to_vec).collect());
    let want: Vec<Vec<u8>> = expected
        .chunks(WIDTH)
        .skip(ys.start)
        .take(ys.len())
        .map(<[u8]>::to_vec)
        .collect();
    assert_eq!(rows, want);
    assert!(!plane.is_mirrored());
}

#[test]
fn rows_stop_at_the_last_row_and_read_a_flat_plane() {
    let (plane, _sharer, expected) = diverged();
    let rows: Vec<&[u8]> = plane.rows(HEIGHT - 2..HEIGHT + 4, WIDTH).collect();
    assert_eq!(rows.len(), 2, "rows past the end are not yielded");
    assert_eq!(rows[1], &expected[(HEIGHT - 1) * WIDTH..]);
    let flat: Plane<u8> = expected.clone().into();
    let all: Vec<u8> = flat.rows(0..HEIGHT, WIDTH).flatten().copied().collect();
    assert_eq!(all, expected);
    assert_eq!(plane.rows(3..3, WIDTH).count(), 0, "an empty range");
}

#[test]
fn read_rect_into_matches_the_slice_and_clips_to_the_plane() {
    let (plane, _sharer, expected) = diverged();
    let (xs, ys) = (2..6, BAND_ROWS - 1..2 * BAND_ROWS + 1);
    let mut out = vec![0; xs.len() * ys.len()];
    assert_no_materialization(|| plane.read_rect_into(xs.clone(), ys.clone(), WIDTH, &mut out));
    assert_eq!(out, rect(&expected, xs, ys));

    // The last band is partial; columns and rows past the plane stay untouched.
    let (xs, ys) = (5..WIDTH + 2, HEIGHT - 1..HEIGHT + 1);
    let mut out = vec![0xAA; xs.len() * ys.len()];
    plane.read_rect_into(xs.clone(), ys, WIDTH, &mut out);
    let last = &expected[(HEIGHT - 1) * WIDTH..];
    assert_eq!(&out[..2], &last[5..]);
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
fn read_into_copies_a_logical_range_across_bands() {
    let (plane, _sharer, expected) = diverged();
    let start = BAND_ROWS * WIDTH - 3;
    let mut out = vec![0; WIDTH * BAND_ROWS + 6];
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
fn for_each_band_reports_every_band_and_its_first_row() {
    let (plane, _sharer, expected) = diverged();
    let mut seen = Vec::new();
    let mut starts = Vec::new();
    assert_no_materialization(|| {
        plane.for_each_band(|y0, band| {
            starts.push(y0);
            seen.extend_from_slice(band);
        })
    });
    assert_eq!(starts, vec![0, BAND_ROWS, 2 * BAND_ROWS]);
    assert_eq!(seen, expected);
}

#[test]
fn rows_mut_forks_only_the_bands_it_touches() {
    let (mut plane, sharer, mut expected) = diverged();
    let before = sharer.as_slice().to_vec();
    let ys = 2 * BAND_ROWS + 1..2 * BAND_ROWS + 3;
    assert_no_materialization(|| {
        for row in plane.rows_mut(ys.clone(), WIDTH) {
            row.fill(0x11);
        }
    });
    for y in ys {
        expected[y * WIDTH..(y + 1) * WIDTH].fill(0x11);
    }
    assert_eq!(plane, expected);
    assert!(
        plane.shares_band(0, &sharer),
        "an untouched band stays shared"
    );
    assert!(!plane.shares_band(2, &sharer), "the written band forked");
    assert_eq!(sharer, before, "the sharer keeps its bytes");
}

#[test]
fn rows_mut_writes_a_unique_base_in_place_and_a_flat_plane() {
    let mut plane: Plane<u8> = pattern(WIDTH * HEIGHT).into();
    plane.fracture(WIDTH);
    let (ptr, _) = plane.base_allocation();
    for row in plane.rows_mut(0..BAND_ROWS + 1, WIDTH) {
        row[0] = 0;
    }
    assert_eq!(
        plane.base_allocation().0,
        ptr,
        "a unique base is written in place"
    );
    assert_eq!(plane.owned_bands().count(), 0);
    assert!((0..BAND_ROWS + 1).all(|y| plane.sample(y * WIDTH) == Some(0)));

    let mut flat: Plane<u8> = pattern(WIDTH * 3).into();
    let rows: Vec<usize> = flat.rows_mut(1..9, WIDTH).map(|r| r.len()).collect();
    assert_eq!(
        rows,
        vec![WIDTH, WIDTH],
        "rows past the end are not yielded"
    );
}

#[test]
fn write_rect_from_round_trips_and_clips() {
    let (mut plane, sharer, mut expected) = diverged();
    let (xs, ys) = (1..5, BAND_ROWS - 2..BAND_ROWS + 2);
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
    assert!(plane.shares_band(2, &sharer));

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
fn for_each_band_mut_reaches_every_sample_without_materializing() {
    let (mut plane, sharer, expected) = diverged();
    let mut starts = Vec::new();
    assert_no_materialization(|| {
        plane.for_each_band_mut(|y0, band| {
            starts.push(y0);
            band.iter_mut().for_each(|b| *b = b.wrapping_add(1));
        })
    });
    assert_eq!(starts, vec![0, BAND_ROWS, 2 * BAND_ROWS]);
    let want: Vec<u8> = expected.iter().map(|b| b.wrapping_add(1)).collect();
    assert_eq!(plane, want);
    assert_ne!(sharer, plane);
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
