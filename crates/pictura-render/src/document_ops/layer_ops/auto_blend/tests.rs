//! Auto-Blend engine tests. Results are image-difference checks: the algorithm
//! is an approximation, so nothing here claims pixel identity with CS6.

use std::sync::atomic::{AtomicBool, AtomicU32};

use pictura_core::{
    BitDepth, ColorMode, Document, Layer, PsdRect, Samples, SmartObject, SourceChannels,
};

use super::sources::{work_scale, MAX_WORK_PIXELS};
use super::*;
use crate::tests::{doc, full, rect, solid};
use crate::{composite_rgba, encode_invert};
use pictura_core::BlendMode;

const STACK: AutoBlendOptions = AutoBlendOptions {
    objective: BlendObjective::StackImages,
    seamless: false,
    fill_transparent: false,
};
const PANORAMA: AutoBlendOptions = AutoBlendOptions {
    objective: BlendObjective::Panorama,
    seamless: false,
    fill_transparent: false,
};

fn plan(d: &Document, paths: &[&str], options: AutoBlendOptions) -> AutoBlendPlan {
    plan_auto_blend(
        d,
        paths,
        options,
        &AtomicBool::new(false),
        &AtomicU32::new(0),
    )
    .expect("not cancelled")
}

fn blend(d: &mut Document, paths: &[&str], options: AutoBlendOptions) -> Vec<String> {
    let planned = plan(d, paths, options);
    apply_auto_blend(d, planned)
}

/// A raster layer over `r` whose gray value comes from document coordinates.
fn painted(name: &str, r: PsdRect, value: impl Fn(i32, i32) -> u8) -> Layer {
    let (w, h) = (r.width() as usize, r.height() as usize);
    let mut data = Vec::with_capacity(w * h);
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            data.push(value(x, y));
        }
    }
    let mut layer = solid(name, r, (0, 0, 0), 255, BlendMode::Normal, 255);
    for id in 0..3 {
        layer.channels[id].data = data.clone().into();
    }
    layer
}

fn gray_of(out: &pictura_core::PixelBuffer, x: u32, y: u32) -> u8 {
    out.data[(y * out.width + x) as usize]
}

fn mask_at(layer: &Layer, x: i32, y: i32) -> u8 {
    let mask = layer.mask.as_ref().expect("a generated mask");
    let i =
        (y - mask.rect.top) as usize * mask.rect.width() as usize + (x - mask.rect.left) as usize;
    mask.data.as_ref().expect("mask data").as_slice()[i]
}

/// Summed `|Laplacian|` over `[x0, x1) × [2, 62)`.
fn focus(sample: impl Fn(i32, i32) -> u8, x0: i32, x1: i32) -> i64 {
    let mut total = 0i64;
    for y in 2..62 {
        for x in x0..x1 {
            let at = |x: i32, y: i32| i64::from(sample(x, y));
            total +=
                (at(x - 1, y) + at(x + 1, y) + at(x, y - 1) + at(x, y + 1) - 4 * at(x, y)).abs();
        }
    }
    total
}

fn checker(x: i32, y: i32) -> u8 {
    if (x / 2 + y / 2) % 2 == 0 {
        90
    } else {
        170
    }
}

/// Each layer sharp where the other is flat.
fn crossed_focus() -> Document {
    let square = full(64, 64);
    doc(
        64,
        64,
        vec![
            painted(
                "Left",
                square,
                |x, y| if x < 32 { checker(x, y) } else { 130 },
            ),
            painted(
                "Right",
                square,
                |x, y| if x >= 32 { checker(x, y) } else { 130 },
            ),
        ],
    )
}

fn pair() -> Document {
    doc(
        20,
        20,
        vec![
            solid(
                "A",
                full(20, 20),
                (100, 100, 100),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "B",
                full(20, 20),
                (120, 120, 120),
                255,
                BlendMode::Normal,
                255,
            ),
        ],
    )
}

fn adjustment(name: &str) -> Layer {
    Layer {
        name: name.into(),
        opacity: 255,
        fill: 255,
        visible: true,
        adjustment: Some(encode_invert()),
        ..Default::default()
    }
}

#[test]
fn auto_blend_refuses_modes() {
    for mode in [
        ColorMode::Lab,
        ColorMode::Cmyk,
        ColorMode::Indexed,
        ColorMode::Bitmap,
    ] {
        let mut d = pair();
        d.mode = mode;
        assert!(
            !can_auto_blend(&d, &["0", "1"]),
            "{mode:?} as a working mode"
        );
        let mut d = pair();
        d.source_mode = Some(mode);
        assert!(
            !can_auto_blend(&d, &["0", "1"]),
            "{mode:?} read from a file"
        );
        let before = d.clone();
        assert!(plan(&d, &["0", "1"], STACK).layers.is_empty());
        assert_eq!(d, before);
    }
    let mut d = pair();
    d.mode = ColorMode::Grayscale;
    assert!(can_auto_blend(&d, &["0", "1"]));
}

#[test]
fn auto_blend_refuses_selected_kinds() {
    let mut d = pair();
    d.layers[0].background = true;
    assert!(!can_auto_blend(&d, &["0", "1"]), "Background");

    let mut d = pair();
    d.layers[1].smart_object = Some(SmartObject::default());
    assert!(!can_auto_blend(&d, &["0", "1"]), "Smart Object");

    let mut d = pair();
    d.layers[1].clipping = true;
    assert!(!can_auto_blend(&d, &["0", "1"]), "clipped");

    assert!(!can_auto_blend(&pair(), &["0"]), "one layer");
    assert!(can_auto_blend(&pair(), &["0", "1"]));
}

#[test]
fn auto_blend_ignores_a_smart_object_outside_the_selection() {
    let mut d = pair();
    let mut smart = solid("S", full(20, 20), (9, 9, 9), 255, BlendMode::Normal, 255);
    smart.smart_object = Some(SmartObject::default());
    d.layers.push(smart).expect("a free slot");
    assert!(can_auto_blend(&d, &["0", "1"]));
    assert!(!can_auto_blend(&d, &["0", "1", "2"]));
}

#[test]
fn auto_blend_skips_ineligible_layers() {
    let mut d = pair();
    d.layers[0].lock = pictura_core::LockFlags::all();
    let mut hidden = solid(
        "Hidden",
        full(20, 20),
        (1, 1, 1),
        255,
        BlendMode::Normal,
        255,
    );
    hidden.visible = false;
    let empty = solid("Empty", full(20, 20), (0, 0, 0), 0, BlendMode::Normal, 255);
    let mut text = solid("Type", full(20, 20), (2, 2, 2), 255, BlendMode::Normal, 255);
    text.type_tool = Some(pictura_core::TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        text: "T".into(),
        bounds: [0; 4],
        text_desc: Vec::new().into(),
        warp_desc: Vec::new().into(),
        fonts: Vec::new(),
        style: None,
        vertical: false,
        box_bounds: None,
        path: None,
    });
    d.layers.push(hidden).expect("slot");
    d.layers.push(empty).expect("slot");
    d.layers.push(text).expect("slot");
    d.layers.push(adjustment("Invert")).expect("slot");
    let blended = plan(&d, &["0", "1", "2", "3", "4", "5"], STACK);
    let paths: Vec<&str> = blended.layers.iter().map(|l| l.path.as_str()).collect();
    assert_eq!(paths, ["0", "1"]);
    assert!(
        !can_auto_blend(&d, &["0", "2", "3", "4", "5"]),
        "one eligible"
    );
}

#[test]
fn stack_images_beats_each_input_focus() {
    let mut d = crossed_focus();
    let inputs = d.clone();
    blend(
        &mut d,
        &["0", "1"],
        AutoBlendOptions {
            seamless: true,
            ..STACK
        },
    );
    assert!(d.layers[0].mask.is_some() && d.layers[1].mask.is_some());
    let out = composite_rgba(&d);
    let score = focus(|x, y| gray_of(&out, x as u32, y as u32), 2, 62);
    for layer in [0, 1] {
        let alone = inputs.layers[layer].channels[0].data.as_slice();
        let input = focus(|x, y| alone[y as usize * 64 + x as usize], 2, 62);
        assert!(score as f64 > input as f64 * 1.4, "{score} vs {input}");
    }
}

#[test]
fn stack_images_prefers_good_exposure() {
    let square = full(32, 32);
    let mut d = doc(
        32,
        32,
        vec![
            painted("Dark", square, |x, _| if x < 16 { 2 } else { 128 }),
            painted("Bright", square, |x, _| if x < 16 { 128 } else { 253 }),
        ],
    );
    blend(&mut d, &["0", "1"], STACK);
    let out = composite_rgba(&d);
    assert!(
        gray_of(&out, 3, 16).abs_diff(128) <= 8,
        "{}",
        gray_of(&out, 3, 16)
    );
    assert!(
        gray_of(&out, 28, 16).abs_diff(128) <= 8,
        "{}",
        gray_of(&out, 28, 16)
    );
}

#[test]
fn panorama_seam_avoids_a_difference() {
    let object = |x: i32, y: i32| (44..52).contains(&x) && (20..30).contains(&y);
    let mut d = doc(
        100,
        50,
        vec![
            painted("A", rect(0, 0, 50, 60), |_, _| 128),
            painted("B", rect(0, 40, 50, 100), move |x, y| {
                if object(x, y) {
                    240
                } else {
                    128
                }
            }),
        ],
    );
    blend(&mut d, &["0", "1"], PANORAMA);
    let out = composite_rgba(&d);
    let values: Vec<u8> = (21..29)
        .flat_map(|y| (45..51).map(move |x| (x, y)))
        .map(|(x, y)| gray_of(&out, x, y))
        .collect();
    let whole_b = values.iter().all(|&v| v >= 220);
    let whole_a = values.iter().all(|&v| v <= 148);
    assert!(whole_a || whole_b, "the object is split: {values:?}");
}

#[test]
fn panorama_keeps_disjoint_layers() {
    let mut d = doc(
        40,
        20,
        vec![
            solid(
                "A",
                rect(0, 0, 20, 18),
                (50, 50, 50),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "B",
                rect(0, 22, 20, 40),
                (200, 200, 200),
                255,
                BlendMode::Normal,
                255,
            ),
        ],
    );
    blend(&mut d, &["0", "1"], PANORAMA);
    assert_eq!(mask_at(&d.layers[0], 5, 5), 255);
    assert_eq!(mask_at(&d.layers[1], 30, 5), 255);
}

#[test]
fn panorama_contained_layer_is_hidden() {
    let mut d = doc(
        40,
        40,
        vec![
            solid(
                "Big",
                full(40, 40),
                (50, 50, 50),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "Small",
                rect(10, 10, 30, 30),
                (200, 200, 200),
                255,
                BlendMode::Normal,
                255,
            ),
        ],
    );
    blend(&mut d, &["0", "1"], PANORAMA);
    assert_eq!(mask_at(&d.layers[1], 20, 20), 0);
    assert_eq!(mask_at(&d.layers[0], 20, 20), 255);
}

#[test]
fn auto_blend_handles_off_canvas_layers() {
    let mut d = doc(
        30,
        20,
        vec![
            solid(
                "A",
                rect(-4, -6, 16, 18),
                (60, 60, 60),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "B",
                rect(0, 12, 20, 40),
                (60, 60, 60),
                255,
                BlendMode::Normal,
                255,
            ),
        ],
    );
    blend(&mut d, &["0", "1"], PANORAMA);
    for i in [0, 1] {
        let layer = &d.layers[i];
        let mask = layer.mask.as_ref().expect("mask");
        assert_eq!(mask.rect, layer.rect);
        assert_eq!(
            mask.data.as_ref().expect("data").len(),
            (layer.rect.width() * layer.rect.height()) as usize
        );
    }
    assert_eq!(
        mask_at(&d.layers[0], -5, 0),
        255,
        "unique content off canvas"
    );
}

#[test]
fn auto_blend_spans_groups() {
    let inner = solid(
        "B",
        rect(0, 10, 20, 30),
        (90, 90, 90),
        255,
        BlendMode::Normal,
        255,
    );
    let group = Layer {
        name: "G".into(),
        opacity: 255,
        fill: 255,
        visible: true,
        is_group: true,
        blend: BlendMode::PassThrough,
        children: pictura_core::LayerStack::from_layers(vec![inner]),
        ..Default::default()
    };
    let mut d = doc(
        30,
        20,
        vec![
            solid(
                "A",
                rect(0, 0, 20, 20),
                (90, 90, 90),
                255,
                BlendMode::Normal,
                255,
            ),
            group,
        ],
    );
    assert!(can_auto_blend(&d, &["0", "1/0"]));
    assert_eq!(blend(&mut d, &["1/0", "0"], PANORAMA), ["0", "1/0"]);
    assert_eq!(mask_at(&d.layers[0], 2, 5), 255);
    assert_eq!(mask_at(&d.layers[1].children[0], 28, 5), 255);

    d.layers[1].visible = false;
    assert!(
        !can_auto_blend(&d, &["0", "1/0"]),
        "hidden through its group"
    );
}

fn native(depth: BitDepth, name: &str, r: PsdRect, value: f32) -> Layer {
    let n = (r.width() * r.height()) as usize;
    let mut layer = solid(
        name,
        r,
        ((value * 255.0) as u8, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    layer.channels[1].data = layer.channels[0].data.clone();
    layer.channels[2].data = layer.channels[0].data.clone();
    let plane = |v: f32| match depth {
        BitDepth::Sixteen => Samples::U16(vec![(v * 65535.0) as u16; n].into()),
        _ => Samples::F32(vec![v; n].into()),
    };
    let store = (0..3)
        .map(|id| (id, plane(value)))
        .chain([(-1, plane(1.0))]);
    layer.source_channels = Some(SourceChannels::new(depth, r, store.collect()));
    layer
}

fn keeps_depth(depth: BitDepth) {
    let mut d = doc(
        20,
        20,
        vec![
            native(depth, "A", full(20, 20), 0.4),
            native(depth, "B", full(20, 20), 0.6),
        ],
    );
    d.source_depth = Some(depth);
    let channels: Vec<_> = d.layers.iter().map(|l| l.source_channels.clone()).collect();
    blend(&mut d, &["0", "1"], STACK);
    assert_eq!(d.source_depth, Some(depth));
    for (layer, before) in d.layers.iter().zip(channels) {
        assert!(layer.mask.is_some());
        assert_eq!(layer.source_channels, before, "planes unchanged");
    }
    let merged = d.layers[2]
        .source_channels
        .as_ref()
        .expect("native samples");
    assert_eq!(merged.depth, depth);
}

#[test]
fn auto_blend_keeps_16_bit_depth() {
    keeps_depth(BitDepth::Sixteen);
}

#[test]
fn auto_blend_keeps_32_bit_depth() {
    keeps_depth(BitDepth::ThirtyTwo);
}

#[test]
fn auto_blend_is_deterministic() {
    let d = crossed_focus();
    assert_eq!(plan(&d, &["0", "1"], STACK), plan(&d, &["0", "1"], STACK));
    assert_eq!(
        plan(&d, &["0", "1"], PANORAMA),
        plan(&d, &["0", "1"], PANORAMA)
    );
}

#[test]
fn auto_blend_scale_stays_within_the_limit() {
    assert_eq!(work_scale(1_000_000, MAX_WORK_PIXELS), 1);
    for (w, h) in [(30_000u64, 30_000u64), (300_000, 300_000), (5_000, 90_000)] {
        let f = work_scale(w * h, MAX_WORK_PIXELS) as u64;
        assert!(w.div_ceil(f) * h.div_ceil(f) <= MAX_WORK_PIXELS * 11 / 10);
    }
}

#[test]
fn auto_blend_cancel_returns_no_plan() {
    let d = crossed_focus();
    let cancel = AtomicBool::new(true);
    let progress = AtomicU32::new(0);
    assert_eq!(
        plan_auto_blend(&d, &["0", "1"], STACK, &cancel, &progress),
        Err(Cancelled)
    );
}

fn mismatched(gray: bool) -> Document {
    let shade = |v: u8| move |x: i32, y: i32| v + ((x + y) % 5) as u8;
    let mut d = doc(
        60,
        20,
        vec![
            painted("A", rect(0, 0, 20, 40), shade(90)),
            painted("B", rect(0, 20, 20, 60), shade(170)),
        ],
    );
    if gray {
        d.mode = ColorMode::Grayscale;
        for layer in d.layers.iter_mut() {
            layer.channels.retain(|c| c.id == 0 || c.id == -1);
        }
    }
    d
}

const SEAMLESS: AutoBlendOptions = AutoBlendOptions {
    objective: BlendObjective::Panorama,
    seamless: true,
    fill_transparent: false,
};

fn seam_step(d: &Document) -> u8 {
    let out = composite_rgba(d);
    gray_of(&out, 5, 10).abs_diff(gray_of(&out, 55, 10))
}

fn narrows_mismatch(gray: bool) {
    let mut d = mismatched(gray);
    let before = seam_step(&d);
    let channels: Vec<Vec<_>> = d.layers.iter().map(|l| l.channels.clone()).collect();
    blend(&mut d, &["0", "1"], SEAMLESS);
    assert_eq!(d.layers.len(), 3, "the sources and the merged layer");
    assert_eq!(d.layers[2].name, "B (merged)");
    assert!(d.layers.iter().all(|l| l.adjustment.is_none()));
    assert!(seam_step(&d) < before / 2, "{} vs {before}", seam_step(&d));
    assert_eq!(d.layers[0].channels, channels[0], "pixels unchanged");
    assert_eq!(d.layers[1].channels, channels[1], "pixels unchanged");
}

#[test]
fn seamless_narrows_a_gain_mismatch() {
    narrows_mismatch(false);
}

#[test]
fn seamless_handles_grayscale() {
    narrows_mismatch(true);
}

#[test]
fn seamless_keeps_matching_tones() {
    let mut d = doc(
        20,
        20,
        vec![
            solid(
                "A",
                full(20, 20),
                (100, 100, 100),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "B",
                full(20, 20),
                (100, 100, 100),
                255,
                BlendMode::Normal,
                255,
            ),
        ],
    );
    blend(&mut d, &["0", "1"], SEAMLESS);
    assert!(gray_of(&composite_rgba(&d), 10, 10).abs_diff(100) <= 1);
}

#[test]
fn stack_images_blends_softly() {
    let square = full(128, 128);
    let mut d = doc(
        128,
        128,
        vec![
            painted("Left", square, |x, y| {
                if x < 64 {
                    checker(x, y) / 2 + 40
                } else {
                    90
                }
            }),
            painted("Right", square, |x, y| {
                if x >= 64 {
                    checker(x, y) / 2 + 100
                } else {
                    150
                }
            }),
        ],
    );
    blend(&mut d, &["0", "1"], STACK);
    let merged = d.layers[2].channels[0].data.as_slice();
    let mean = |x: usize| {
        (0..128)
            .map(|y| i32::from(merged[y * 128 + x]))
            .sum::<i32>()
            / 128
    };
    let steps = (32..95).map(|x| (mean(x + 4) - mean(x)).abs());
    assert!(steps.max().unwrap_or(0) < 25, "a hard step between regions");
}

#[test]
fn merged_sits_above_a_clipped_run() {
    let a = painted("A", rect(0, 0, 20, 40), |_, _| 90);
    let b = painted("B", rect(0, 20, 20, 60), |_, _| 170);
    let mut clipped = adjustment("Mine");
    clipped.clipping = true;
    let group = Layer {
        name: "G".into(),
        opacity: 255,
        fill: 255,
        visible: true,
        is_group: true,
        blend: BlendMode::PassThrough,
        children: pictura_core::LayerStack::from_layers(vec![a, b, clipped]),
        ..Default::default()
    };
    let top = solid("Top", full(60, 20), (0, 0, 0), 0, BlendMode::Normal, 255);
    let mut d = doc(60, 20, vec![group, top]);
    let paths = blend(&mut d, &["0/0", "0/1"], SEAMLESS);
    let names: Vec<&str> = d.layers[0]
        .children
        .iter()
        .map(|l| l.name.as_str())
        .collect();
    assert_eq!(names, ["A", "B", "Mine", "B (merged)"]);
    assert_eq!(paths, ["0/0", "0/1"]);
}

#[test]
fn rerun_replaces_the_merged_layer() {
    let mut d = mismatched(false);
    let paths = blend(&mut d, &["0", "1"], SEAMLESS);
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    blend(&mut d, &refs, PANORAMA);
    assert_eq!(d.layers.len(), 3, "merged layers do not stack");
}

#[test]
fn rerun_keeps_a_renamed_merged_layer() {
    let mut d = mismatched(false);
    blend(&mut d, &["0", "1"], SEAMLESS);
    d.layers[2].name = "Mine".into();
    blend(&mut d, &["0", "1"], PANORAMA);
    let names: Vec<&str> = d.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["A", "B", "B (merged)", "Mine"]);
}

const FILLED: AutoBlendOptions = AutoBlendOptions {
    fill_transparent: true,
    ..PANORAMA
};

fn gapped() -> Document {
    doc(
        40,
        40,
        vec![
            solid(
                "A",
                rect(0, 0, 40, 15),
                (200, 40, 40),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "B",
                rect(0, 25, 40, 40),
                (200, 40, 40),
                255,
                BlendMode::Normal,
                255,
            ),
        ],
    )
}

#[test]
fn content_aware_fill_covers_the_canvas() {
    let mut d = gapped();
    blend(&mut d, &["0", "1"], FILLED);
    let merged = &d.layers[2];
    assert_eq!(merged.rect, full(40, 40));
    let alpha = merged.channels[3].data.as_slice();
    assert!(alpha.iter().all(|&a| a == 255), "opaque across the canvas");
    assert_eq!(merged.channels[0].data.as_slice()[20 * 40 + 20], 200);
}

#[test]
fn content_aware_fill_is_ignored_at_high_depth() {
    let mut d = doc(
        40,
        40,
        vec![
            native(BitDepth::Sixteen, "A", rect(0, 0, 40, 15), 0.4),
            native(BitDepth::Sixteen, "B", rect(0, 25, 40, 40), 0.4),
        ],
    );
    d.source_depth = Some(BitDepth::Sixteen);
    blend(&mut d, &["0", "1"], FILLED);
    assert_eq!(d.layers[2].channels[3].data.as_slice()[20 * 40 + 20], 0);
}

fn retained_mask_layer() -> Document {
    let mut layer = native(BitDepth::Sixteen, "A", full(4, 4), 0.5);
    let old = Samples::U16(vec![0u16; 16].into());
    if let Some(store) = layer.source_channels.as_mut() {
        store.planes.insert(0, (-2, old));
    }
    layer.mask = Some(LayerMask {
        rect: full(4, 4),
        data: Some(vec![0u8; 16].into()),
        ..Default::default()
    });
    let mut d = doc(4, 4, vec![layer]);
    d.source_depth = Some(BitDepth::Sixteen);
    d
}

fn revealed() -> LayerMask {
    LayerMask {
        rect: full(4, 4),
        data: Some(vec![255u8; 16].into()),
        ..Default::default()
    }
}

#[test]
fn replace_layer_mask_drops_the_native_plane() {
    let mut d = retained_mask_layer();
    assert_eq!(composite_rgba(&d).data[3 * 16], 0, "the old mask gates");
    let undo = d.clone();
    assert!(super::super::replace_layer_mask(&mut d, "0", revealed()));
    assert_eq!(composite_rgba(&d).data[3 * 16], 255, "the new mask gates");
    assert_eq!(
        composite_rgba(&undo).data[3 * 16],
        0,
        "a snapshot keeps the old one"
    );
    assert!(!super::super::replace_layer_mask(&mut d, "9", revealed()));
}

#[test]
fn replace_layer_mask_round_trips() {
    let mut d = retained_mask_layer();
    assert!(super::super::replace_layer_mask(&mut d, "0", revealed()));
    let bytes = pictura_codec::write_psd(&d).expect("written");
    let back = pictura_codec::read_psd(&bytes).expect("read");
    assert_eq!(
        composite_rgba(&back).data[3 * 16],
        255,
        "the reload keeps the new mask"
    );
}

/// A bright flat base, and above it a darker flat layer with a detailed square.
fn subject_over_sky() -> Document {
    let square = full(96, 96);
    let subject = |x: i32, y: i32| {
        if (32..64).contains(&x) && (32..64).contains(&y) {
            checker(x, y)
        } else {
            60
        }
    };
    doc(
        96,
        96,
        vec![
            painted("Sky", square, |_, _| 215),
            painted("Subject", square, subject),
        ],
    )
}

#[test]
fn stack_images_drops_a_flat_upper_background() {
    let mut d = subject_over_sky();
    blend(&mut d, &["0", "1"], STACK);
    assert_eq!(
        mask_at(&d.layers[1], 6, 6),
        0,
        "flat areas stay with the base"
    );
    assert_eq!(mask_at(&d.layers[1], 48, 48), 255, "the detail is kept");
}

#[test]
fn stack_images_keeps_upper_detail_over_busy_texture() {
    let square = full(96, 96);
    let noise = |x: i32, y: i32| ((x * 7 + y * 13) * 37 % 97 + 80) as u8;
    let mut d = doc(
        96,
        96,
        vec![
            painted("Spray", square, noise),
            painted("Subject", square, |x, y| {
                if (x / 8 + y / 8) % 2 == 0 {
                    60
                } else {
                    190
                }
            }),
        ],
    );
    blend(&mut d, &["0", "1"], STACK);
    assert_eq!(mask_at(&d.layers[1], 48, 48), 255);
}

#[test]
fn seamless_stack_takes_tone_from_the_base() {
    const TONED: AutoBlendOptions = AutoBlendOptions {
        seamless: true,
        ..STACK
    };
    let mut d = subject_over_sky();
    blend(&mut d, &["0", "1"], TONED);
    let merged = d.layers[2].channels[0].data.as_slice();
    let mean: u32 = (40..56)
        .flat_map(|y| (40..56).map(move |x| u32::from(merged[y * 96 + x])))
        .sum::<u32>()
        / 256;
    assert!(mean > 160, "broad tone follows the bright base: {mean}");
    let spread = (40..56).map(|x| merged[48 * 96 + x]);
    let (low, high) = (spread.clone().min(), spread.max());
    assert!(
        high.zip(low).is_some_and(|(h, l)| h - l > 30),
        "detail survives"
    );
}

#[test]
fn seamless_stack_keeps_the_subject_colour() {
    let square = full(96, 96);
    let inside = |x: i32, y: i32| (32..64).contains(&x) && (32..64).contains(&y);
    let mut subject = painted("Subject", square, move |x, y| {
        if inside(x, y) {
            checker(x, y)
        } else {
            60
        }
    });
    let red: Vec<u8> = (0..96 * 96)
        .map(|i| {
            let (x, y) = (i % 96, i / 96);
            if inside(x, y) {
                230
            } else {
                60
            }
        })
        .collect();
    subject.channels[0].data = red.into();
    let sky = solid("Sky", square, (40, 120, 230), 255, BlendMode::Normal, 255);
    let mut d = doc(96, 96, vec![sky, subject]);
    blend(
        &mut d,
        &["0", "1"],
        AutoBlendOptions {
            seamless: true,
            ..STACK
        },
    );
    let at = |id: usize| i32::from(d.layers[2].channels[id].data.as_slice()[48 * 96 + 48]);
    assert!(
        at(0) > at(2) + 40,
        "red over a blue base: {} vs {}",
        at(0),
        at(2)
    );
}

#[test]
fn stack_focus_stops_at_subject_edges() {
    let mut d = subject_over_sky();
    blend(&mut d, &["0", "1"], STACK);
    // A soft transition remains; a focus leak read 234 here.
    let beside = mask_at(&d.layers[1], 26, 48);
    assert!(beside < 32, "no halo beside the subject: {beside}");
}
