use std::collections::VecDeque;

use super::{dilate, erode, flood, flood_peak, median, Border};

fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn random_plane(w: usize, h: usize, seed: u64) -> Vec<u8> {
    let mut state = seed | 1;
    (0..w * h)
        .map(|_| (next_u64(&mut state) & 0xff) as u8)
        .collect()
}

fn naive_morph<T: Copy + PartialOrd>(
    src: &[T],
    w: usize,
    h: usize,
    radius: usize,
    border: Border<T>,
    want_max: bool,
) -> Vec<T> {
    let r = radius as isize;
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let mut best: Option<T> = None;
            for dy in -r..=r {
                for dx in -r..=r {
                    let xx = x as isize + dx;
                    let yy = y as isize + dy;
                    let value = if xx < 0 || yy < 0 || xx >= w as isize || yy >= h as isize {
                        match border {
                            Border::Zero(value) => value,
                            Border::Clamp => {
                                let cx = xx.clamp(0, w as isize - 1) as usize;
                                let cy = yy.clamp(0, h as isize - 1) as usize;
                                src[cy * w + cx]
                            }
                        }
                    } else {
                        src[yy as usize * w + xx as usize]
                    };
                    best = Some(match best {
                        None => value,
                        Some(acc) => {
                            if want_max {
                                if value > acc {
                                    value
                                } else {
                                    acc
                                }
                            } else if value < acc {
                                value
                            } else {
                                acc
                            }
                        }
                    });
                }
            }
            out.push(best.expect("non-empty window"));
        }
    }
    out
}

fn naive_median(src: &[u8], w: usize, h: usize, radius: usize) -> Vec<u8> {
    let r = radius as isize;
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let mut window = Vec::with_capacity((2 * radius + 1).pow(2));
            for dy in -r..=r {
                for dx in -r..=r {
                    let xx = (x as isize + dx).clamp(0, w as isize - 1) as usize;
                    let yy = (y as isize + dy).clamp(0, h as isize - 1) as usize;
                    window.push(src[yy * w + xx]);
                }
            }
            window.sort_unstable();
            out.push(window[window.len() / 2]);
        }
    }
    out
}

fn naive_flood(
    w: usize,
    h: usize,
    seeds: &[u8],
    matches: impl Fn(usize, usize) -> bool,
) -> Option<Vec<u8>> {
    let n = w * h;
    if n == 0 || seeds.len() < n || !seeds[..n].iter().any(|&v| v != 0) {
        return None;
    }
    let mut mask = vec![0u8; n];
    let mut queue = VecDeque::new();
    for (p, &seed) in seeds[..n].iter().enumerate() {
        if seed != 0 {
            mask[p] = 255;
            queue.push_back(p);
        }
    }
    while let Some(p) = queue.pop_front() {
        let x = p % w;
        let y = p / w;
        let neighbours = [
            x.checked_sub(1).map(|nx| (nx, y)),
            (x + 1 < w).then_some((x + 1, y)),
            y.checked_sub(1).map(|ny| (x, ny)),
            (y + 1 < h).then_some((x, y + 1)),
        ];
        for (nx, ny) in neighbours.into_iter().flatten() {
            let q = ny * w + nx;
            if mask[q] == 0 && matches(p, q) {
                mask[q] = 255;
                queue.push_back(q);
            }
        }
    }
    Some(mask)
}

#[test]
fn morph_radius_zero_is_the_identity() {
    let src = random_plane(7, 5, 0x51);
    for border in [Border::Zero(0u8), Border::Clamp] {
        assert_eq!(dilate(&src, 7, 5, 0, border), src);
        assert_eq!(erode(&src, 7, 5, 0, border), src);
    }
}

#[test]
fn morph_zero_border_erodes_from_the_edge() {
    let src = vec![255u8; 6 * 5];
    let out = erode(&src, 6, 5, 2, Border::Zero(0u8));
    assert_eq!(out[2 * 6], 0, "left border is eroded");
    assert_eq!(out[2 * 6 + 5], 0, "right border is eroded");
    assert_eq!(out[3], 0, "top border is eroded");
    assert_eq!(out[2 * 6 + 2], 255, "the interior survives");
}

#[test]
fn morph_clamp_border_keeps_the_edge() {
    let src = vec![255u8; 6 * 5];
    assert_eq!(erode(&src, 6, 5, 2, Border::Clamp), src);
}

#[test]
fn morph_matches_naive_at_every_radius_and_border() {
    let sizes = [(1usize, 1usize), (3, 1), (1, 4), (7, 5), (9, 4)];
    for (w, h) in sizes {
        let src = random_plane(w, h, (w * 31 + h * 17) as u64);
        for radius in [1usize, 2, 3, 100] {
            for border in [Border::Zero(0u8), Border::Clamp] {
                assert_eq!(
                    dilate(&src, w, h, radius, border),
                    naive_morph(&src, w, h, radius, border, true),
                    "dilate {w}x{h} r={radius} {border:?}"
                );
                assert_eq!(
                    erode(&src, w, h, radius, border),
                    naive_morph(&src, w, h, radius, border, false),
                    "erode {w}x{h} r={radius} {border:?}"
                );
            }
        }
    }
}

#[test]
fn median_matches_the_sorted_window() {
    let sizes = [(1usize, 1usize), (2, 4), (1, 3), (7, 5), (9, 6)];
    for (w, h) in sizes {
        let src = random_plane(w, h, (w * 13 + h * 7 + 1) as u64);
        for radius in [1usize, 2, 3, 5, 100, 128] {
            assert_eq!(
                median(&src, w, h, radius),
                naive_median(&src, w, h, radius),
                "median {w}x{h} r={radius}"
            );
        }
    }
    let (w, h) = (300usize, 40usize);
    let src = random_plane(w, h, 0x5151);
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("pool")
        .install(|| {
            assert_eq!(
                median(&src, w, h, 5),
                naive_median(&src, w, h, 5),
                "median {w}x{h} r=5 strips"
            );
        });
}

#[test]
fn median_radius_zero_is_the_identity() {
    let src = random_plane(7, 5, 0x93);
    assert_eq!(median(&src, 7, 5, 0), src);
}

#[test]
fn median_binary_majority_removes_and_fills() {
    let mut isolated = vec![0u8; 9 * 9];
    isolated[4 * 9 + 4] = 255;
    assert_eq!(median(&isolated, 9, 9, 1)[4 * 9 + 4], 0);

    let mut hole = vec![255u8; 11 * 11];
    hole[5 * 11 + 5] = 0;
    assert_eq!(median(&hole, 11, 11, 1)[5 * 11 + 5], 255);
}

#[test]
#[allow(clippy::needless_borrows_for_generic_args)]
fn flood_matches_the_naive_queue() {
    let (w, h) = (9usize, 7usize);
    let mut state = 0x1234_5678u64;
    for trial in 0..20 {
        let mut seeds = vec![0u8; w * h];
        for slot in &mut seeds {
            if next_u64(&mut state).is_multiple_of(5) {
                *slot = 1;
            }
        }
        let gate: Vec<u8> = (0..w * h)
            .map(|_| (next_u64(&mut state) & 0xff) as u8)
            .collect();
        let predicate =
            |from: usize, to: usize| (gate[to] as u32 + trial) % 4 <= gate[from] as u32 % 4;
        assert_eq!(
            flood(w, h, &seeds, &predicate),
            naive_flood(w, h, &seeds, &predicate),
            "trial {trial}"
        );
    }
}

#[test]
#[allow(clippy::needless_borrows_for_generic_args)]
fn flood_multi_seed_matches_the_naive_queue() {
    let (w, h) = (6usize, 5usize);
    let mut seeds = vec![0u8; w * h];
    for (x, y) in [(0usize, 0usize), (5, 0), (0, 4), (3, 2)] {
        seeds[y * w + x] = 255;
    }
    let predicate = |_: usize, to: usize| to % 7 != 3;
    assert_eq!(
        flood(w, h, &seeds, &predicate),
        naive_flood(w, h, &seeds, &predicate)
    );
}

#[test]
fn flood_stops_at_the_predicate_boundary() {
    let (w, h) = (5usize, 5usize);
    let mut seeds = vec![0u8; w * h];
    seeds[w + 1] = 1;
    let allowed: Vec<bool> = (0..w * h).map(|p| p % w < 3).collect();
    let mask = flood(w, h, &seeds, |_, to| allowed[to]).expect("one seed");
    for y in 0..h {
        for x in 0..w {
            assert_eq!(mask[y * w + x], if x < 3 { 255 } else { 0 }, "({x},{y})");
        }
    }
}

#[test]
fn flood_frontier_is_bounded_by_spans() {
    let (w, h) = (1000usize, 1000usize);
    let mut seeds = vec![0u8; w * h];
    seeds[500 * w + 500] = 1;
    let (mask, peak, visited) = flood_peak(w, h, &seeds, |_, _| true).expect("one seed");
    assert!(mask.iter().all(|&v| v == 255), "the plane floods");
    assert!(
        visited == mask.iter().filter(|&&v| v == 255).count(),
        "every pixel is first-marked once"
    );
    assert!(peak <= 2 * h, "peak frontier {peak} exceeds 2h");
}

#[test]
fn flood_seed_runs_are_visited_once() {
    let (w, h) = (7usize, 1usize);
    let mut seeds = vec![0u8; w * h];
    seeds[0] = 1;
    seeds[2] = 1;
    let (mask, peak, visited) = flood_peak(w, h, &seeds, |_, to| to < 3).expect("two seeds");
    assert_eq!(mask, vec![255, 255, 255, 0, 0, 0, 0]);
    assert_eq!(visited, 3, "the absorbed seed run is not counted twice");
    assert_eq!(peak, 1);
}

#[test]
fn flood_rejects_an_out_of_plane_seed() {
    assert!(flood(4, 4, &[0u8; 16], |_, _| true).is_none());
    assert!(flood(4, 4, &[0u8; 3], |_, _| true).is_none());
}

#[test]
fn kernels_agree_under_one_and_many_threads() {
    let (w, h) = (41usize, 33usize);
    let src = random_plane(w, h, 0xabcdef);
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("pool")
        .install(|| {
            (
                dilate(&src, w, h, 7, Border::Zero(0u8)),
                erode(&src, w, h, 7, Border::Clamp),
                median(&src, w, h, 9),
            )
        });
    let many = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("pool")
        .install(|| {
            (
                dilate(&src, w, h, 7, Border::Zero(0u8)),
                erode(&src, w, h, 7, Border::Clamp),
                median(&src, w, h, 9),
            )
        });
    assert_eq!(one, many);
}
