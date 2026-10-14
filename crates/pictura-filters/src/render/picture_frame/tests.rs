//! The ported property tests for Picture Frame.

use super::geometry::{fill, place};
use super::shapes::{flower_shape, leaf_shape};
use super::*;
use crate::photorust::pixmap::{Pixmap, Rgba8};

fn white(w: u32, h: u32) -> Pixmap {
    Pixmap::filled(w, h, Rgba8::new(255, 255, 255, 255))
}

fn changed(a: &Pixmap, b: &Pixmap) -> usize {
    a.as_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_bytes().as_chunks::<4>().0.iter())
        .filter(|(p, q)| p != q)
        .count()
}

#[test]
fn every_frame_draws_something_and_leaves_the_middle_alone() {
    for frame in FRAMES {
        let before = white(300, 200);
        let mut px = before.clone();
        picture_frame(
            &mut px,
            &FrameOptions {
                frame,
                ..FrameOptions::default()
            },
        );
        assert!(
            changed(&before, &px) > 200,
            "frame {frame} drew next to nothing"
        );
        // The middle of the picture is the picture.
        for y in 80..120 {
            for x in 120..180 {
                assert_eq!(
                    px.get(x, y),
                    Rgba8::new(255, 255, 255, 255),
                    "frame {frame} reached {x},{y}"
                );
            }
        }
    }
}

#[test]
fn every_flower_and_leaf_has_a_shape() {
    for flower in 1..=22 {
        let mut px = white(64, 64);
        fill(
            &mut px,
            &place(&flower_shape(flower), 32.0, 32.0, 28.0, 0.0),
            Rgba8::BLACK,
            1.0,
        );
        let ink = px
            .as_bytes()
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] < 128)
            .count();
        assert!(
            ink > 100 && ink < 64 * 64 * 9 / 10,
            "flower {flower} covers {ink} pixels"
        );
    }
    for leaf in 1..=23 {
        let mut px = white(64, 64);
        fill(
            &mut px,
            &place(&leaf_shape(leaf), 32.0, 32.0, 28.0, 0.0),
            Rgba8::BLACK,
            1.0,
        );
        let ink = px
            .as_bytes()
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] < 128)
            .count();
        assert!(
            ink > 100 && ink < 64 * 64 * 9 / 10,
            "leaf {leaf} covers {ink} pixels"
        );
    }
}

#[test]
fn a_hole_is_cut_out_of_the_shape_around_it() {
    // Star Cloud is a ring with a star inside it: its middle, between the
    // two, is left bare.
    let mut px = white(100, 100);
    fill(
        &mut px,
        &place(&flower_shape(16), 50.0, 50.0, 45.0, 0.0),
        Rgba8::BLACK,
        1.0,
    );
    assert_eq!(px.get(50, 50).r, 0, "the star is missing");
    assert_eq!(px.get(50, 50 + 29).r, 255, "the ring is filled in");
    assert_eq!(px.get(50, 50 + 42).r, 0, "the ring is missing");
}

#[test]
fn the_frame_draws_in_the_colours_it_is_given() {
    let mut px = white(300, 200);
    let options = FrameOptions {
        frame: 34,
        vine: [200, 0, 0],
        flower_colour: [0, 0, 200],
        ..FrameOptions::default()
    };
    picture_frame(&mut px, &options);
    let has = |c: [u8; 3]| {
        px.as_bytes()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[..3] == [c[0], c[1], c[2]])
    };
    assert!(has(options.vine), "no vine in the vine colour");
    assert!(
        has(options.flower_colour),
        "no flowers in the flower colour"
    );
}

#[test]
fn a_frame_without_leaves_ignores_the_leaf() {
    let run = |leaf| {
        let mut px = white(240, 160);
        picture_frame(
            &mut px,
            &FrameOptions {
                frame: 34,
                leaf,
                ..FrameOptions::default()
            },
        );
        px
    };
    assert_eq!(run(4).as_bytes(), run(20).as_bytes());
}

#[test]
fn a_bigger_margin_moves_the_frame_in() {
    let first_ink = |margin| {
        let mut px = white(300, 200);
        picture_frame(
            &mut px,
            &FrameOptions {
                frame: 36,
                margin,
                ..FrameOptions::default()
            },
        );
        (0..100).find(|&y| px.get(150, y).r < 255).unwrap()
    };
    assert!(first_ink(40) > first_ink(5));
}

#[test]
fn the_arrangement_reshuffles_the_frame() {
    let run = |arrangement| {
        let mut px = white(240, 160);
        picture_frame(
            &mut px,
            &FrameOptions {
                frame: 22,
                arrangement,
                ..FrameOptions::default()
            },
        );
        px
    };
    assert_eq!(run(3).as_bytes(), run(3).as_bytes());
    assert_ne!(run(3).as_bytes(), run(4).as_bytes());
}

#[test]
fn fade_thins_the_middle_of_each_side_and_spares_the_corners() {
    let ink = |fade, x0: i32, x1: i32| {
        let mut px = white(400, 300);
        picture_frame(
            &mut px,
            &FrameOptions {
                frame: 36,
                fade,
                ..FrameOptions::default()
            },
        );
        (x0..x1)
            .flat_map(|x| (0..40).map(move |y| (x, y)))
            .map(|(x, y)| 255 - px.get(x, y).g as u32)
            .sum::<u32>()
    };
    // The top side's middle, and its left end.
    let (middle, corner) = ((180, 220), (8, 48));
    assert!(
        ink(80, middle.0, middle.1) * 3 < ink(0, middle.0, middle.1),
        "the middle did not fade"
    );
    let (plain, faded) = (ink(0, corner.0, corner.1), ink(80, corner.0, corner.1));
    assert!(
        faded * 10 > plain * 7,
        "the corner faded from {plain} to {faded}"
    );
}

#[test]
fn thickness_widens_the_lines() {
    let ink = |thickness| {
        let mut px = white(300, 200);
        picture_frame(
            &mut px,
            &FrameOptions {
                frame: 17,
                thickness,
                ..FrameOptions::default()
            },
        );
        changed(&white(300, 200), &px)
    };
    assert!(ink(120) > ink(10));
}

#[test]
fn number_of_lines_packs_the_lines_closer() {
    let crossings = |lines| {
        let mut px = white(400, 300);
        let options = FrameOptions {
            frame: 14,
            lines,
            flower: 0,
            ..FrameOptions::default()
        };
        picture_frame(&mut px, &options);
        // Along the middle of the top band.
        let y = (0..60)
            .max_by_key(|&y| (0..400).filter(|&x| px.get(x, y).g < 200).count())
            .unwrap();
        (1..400)
            .filter(|&x| px.get(x, y).g < 200 && px.get(x - 1, y).g >= 200)
            .count()
    };
    assert!(crossings(15) > crossings(5) * 2);
}

#[test]
fn invert_turns_a_frame_inside_out() {
    for frame in [1, 20, 38, 44] {
        let run = |invert| {
            let mut px = white(240, 160);
            picture_frame(
                &mut px,
                &FrameOptions {
                    frame,
                    invert,
                    ..FrameOptions::default()
                },
            );
            px
        };
        assert_ne!(run(false).as_bytes(), run(true).as_bytes(), "frame {frame}");
    }
}

#[test]
fn the_moulding_is_lit_from_the_top_left() {
    // The bead's light and dark sides swap between the top edge and the
    // bottom one, which is what makes it read as raised.
    let mut px = white(300, 300);
    let colour = [150, 100, 50];
    picture_frame(
        &mut px,
        &FrameOptions {
            frame: 42,
            vine: colour,
            ..FrameOptions::default()
        },
    );
    let column = |ys: std::ops::Range<i32>| -> Vec<u8> { ys.map(|y| px.get(150, y).r).collect() };
    let top = column(0..60);
    let bottom: Vec<u8> = column(240..300).into_iter().rev().collect();
    assert_ne!(top, bottom);
}

#[test]
fn out_of_range_options_are_rejected() {
    let bad = [
        FrameOptions {
            frame: 0,
            ..FrameOptions::default()
        },
        FrameOptions {
            frame: 48,
            ..FrameOptions::default()
        },
        FrameOptions {
            flower: 23,
            ..FrameOptions::default()
        },
        FrameOptions {
            leaf: 24,
            ..FrameOptions::default()
        },
        FrameOptions {
            margin: 101,
            ..FrameOptions::default()
        },
        FrameOptions {
            thickness: 0,
            ..FrameOptions::default()
        },
    ];
    for options in bad {
        assert!(options.validate().is_err(), "{options:?} was accepted");
    }
    assert!(FrameOptions::default().validate().is_ok());
}
