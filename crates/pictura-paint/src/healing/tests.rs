//! Healing engine checks: the fills, the mode/type matrix, determinism, the
//! refusal contracts, and the layer-level wrapper. Adapted from photorust's
//! `healing.rs` tests onto this crate's `RgbaImage`/`Document` API.

use super::*;
use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LockFlags,
};

const BASE: [u8; 4] = [100, 120, 140, 255];

fn flat(w: i32, h: i32, px: [u8; 4]) -> RgbaImage {
    RgbaImage {
        width: w,
        height: h,
        data: vec![px; (w * h) as usize],
    }
}

fn rect(w: i32, h: i32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: h,
        right: w,
    }
}

/// Coverage of a disc centred in a `w`×`h` region.
fn disc(region: PsdRect, radius: f32) -> Vec<f32> {
    let (cx, cy) = (
        (region.left + region.right) as f32 / 2.0,
        (region.top + region.bottom) as f32 / 2.0,
    );
    (region.top..region.bottom)
        .flat_map(|y| (region.left..region.right).map(move |x| (x, y)))
        .map(|(x, y)| {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            (radius - d + 0.5).clamp(0.0, 1.0)
        })
        .collect()
}

fn all_ones(region: PsdRect) -> Vec<f32> {
    vec![1.0; (region.width() * region.height()) as usize]
}

fn px(img: &RgbaImage, x: i32, y: i32) -> [u8; 4] {
    img.data[(y * img.width + x) as usize]
}

#[test]
fn a_blemish_on_a_flat_field_disappears() {
    let mut img = flat(32, 32, BASE);
    for y in 12..20 {
        for x in 12..20 {
            img.set(x, y, [255, 255, 255, 255]);
        }
    }
    let region = PsdRect {
        top: 12,
        left: 12,
        bottom: 20,
        right: 20,
    };
    let done = heal_region(
        &mut img,
        region,
        &all_ones(region),
        HealMode::ProximityMatch,
    );
    assert_eq!(done, Some(region));
    // Every healed pixel returns to the flat field; the blemish is gone.
    for y in 12..20 {
        for x in 12..20 {
            assert_eq!(px(&img, x, y), BASE, "blemish survived at ({x}, {y})");
        }
    }
}

#[test]
fn proximity_match_continues_a_gradient() {
    let (w, h) = (48, 16);
    let mut img = flat(w, h, BASE);
    for y in 0..h {
        for x in 0..w {
            let v = (x * 255 / (w - 1)) as u8;
            img.set(x, y, [v, v, v, 255]);
        }
    }
    let original = img.clone();
    let region = PsdRect {
        top: 4,
        left: 20,
        bottom: 12,
        right: 28,
    };
    heal_region(
        &mut img,
        region,
        &all_ones(region),
        HealMode::ProximityMatch,
    );
    // The fill follows the horizontal ramp: monotone in x, close to the original.
    for y in 4..12 {
        for x in 21..28 {
            let here = px(&img, x, y)[0] as i32;
            let left = px(&img, x - 1, y)[0] as i32;
            assert!(here >= left, "gradient dipped at ({x}, {y})");
            let want = px(&original, x, y)[0] as i32;
            assert!((here - want).abs() <= 2, "gradient off at ({x}, {y})");
        }
    }
}

#[test]
fn content_aware_carries_an_edge_across_the_hole() {
    // A vertical edge at x=24; the hole straddles it, so the fill must keep a
    // brightness step rather than flatten it.
    let (w, h) = (48, 32);
    let mut img = flat(w, h, [20, 20, 20, 255]);
    for y in 0..h {
        for x in 24..w {
            img.set(x, y, [220, 220, 220, 255]);
        }
    }
    let region = PsdRect {
        top: 12,
        left: 20,
        bottom: 20,
        right: 28,
    };
    heal_region(&mut img, region, &all_ones(region), HealMode::ContentAware);
    // Left of the edge stays dark, right stays light: the step is preserved.
    let dark = px(&img, 22, 16)[0] as i32;
    let light = px(&img, 26, 16)[0] as i32;
    assert!(
        light - dark > 100,
        "edge collapsed: dark={dark} light={light}"
    );
}

#[test]
fn create_texture_adds_grain_where_the_surroundings_have_grain() {
    let (w, h) = (40, 40);
    let mut img = flat(w, h, BASE);
    // Noisy surroundings, deterministic noise.
    let mut seed = 1u32;
    for y in 0..h {
        for x in 0..w {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (seed >> 24) as i32 - 128;
            let v = (120 + n / 4).clamp(0, 255) as u8;
            img.set(x, y, [v, v, v, 255]);
        }
    }
    let region = PsdRect {
        top: 16,
        left: 16,
        bottom: 24,
        right: 24,
    };
    heal_region(&mut img, region, &all_ones(region), HealMode::CreateTexture);
    // The fill is not a flat block: it varies like its surroundings.
    let mut min = 255u8;
    let mut max = 0u8;
    for y in 16..24 {
        for x in 16..24 {
            let v = px(&img, x, y)[0];
            min = min.min(v);
            max = max.max(v);
        }
    }
    assert!(max - min >= 4, "texture fill is flat: {}..{}", min, max);
}

#[test]
fn healing_leaves_pixels_outside_the_brush_alone() {
    let mut img = flat(32, 32, BASE);
    img.set(11, 16, [1, 2, 3, 255]);
    img.set(20, 16, [4, 5, 6, 255]);
    let region = PsdRect {
        top: 12,
        left: 12,
        bottom: 20,
        right: 20,
    };
    heal_region(
        &mut img,
        region,
        &all_ones(region),
        HealMode::ProximityMatch,
    );
    assert_eq!(px(&img, 11, 16), [1, 2, 3, 255]);
    assert_eq!(px(&img, 20, 16), [4, 5, 6, 255]);
}

#[test]
fn partial_coverage_blends_rather_than_replacing() {
    let mut img = flat(32, 32, [0, 0, 0, 255]);
    // A white 4×4 patch in the middle of black; half coverage keeps it grey.
    for y in 14..18 {
        for x in 14..18 {
            img.set(x, y, [255, 255, 255, 255]);
        }
    }
    let region = PsdRect {
        top: 14,
        left: 14,
        bottom: 18,
        right: 18,
    };
    heal_region(&mut img, region, &[0.5; 16], HealMode::ProximityMatch);
    let v = px(&img, 16, 16)[0];
    assert!((1..=254).contains(&v), "expected a blend, got {v}");
}

#[test]
fn an_empty_coverage_mask_does_nothing() {
    let mut img = flat(32, 32, BASE);
    let before = img.clone();
    let region = PsdRect {
        top: 12,
        left: 12,
        bottom: 20,
        right: 20,
    };
    let done = heal_region(&mut img, region, &vec![0.0; 64], HealMode::ProximityMatch);
    assert_eq!(done, None);
    assert_eq!(img, before);
}

#[test]
fn a_hole_with_no_surroundings_is_refused() {
    // Coverage is the whole (canvas-sized) region: no boundary to read from.
    let mut img = flat(16, 16, BASE);
    let before = img.clone();
    let region = rect(16, 16);
    let done = heal_region(
        &mut img,
        region,
        &all_ones(region),
        HealMode::ProximityMatch,
    );
    assert_eq!(done, None);
    assert_eq!(img, before);
}

#[test]
fn healing_is_deterministic() {
    let (w, h) = (40, 40);
    let mut a = flat(w, h, BASE);
    for y in 0..h {
        for x in 0..w {
            let v = ((x * 7 + y * 13) % 256) as u8;
            a.set(x, y, [v, v, v, 255]);
        }
    }
    let mut b = a.clone();
    let region = PsdRect {
        top: 12,
        left: 12,
        bottom: 28,
        right: 28,
    };
    let cov = disc(region, 6.0);
    // Content-Aware is seeded: the same stroke must heal the same way twice.
    heal_region(&mut a, region, &cov, HealMode::ContentAware);
    heal_region(&mut b, region, &cov, HealMode::ContentAware);
    assert_eq!(a, b);
}

#[test]
fn a_zero_offset_clone_is_a_no_op() {
    let mut img = flat(32, 32, BASE);
    let before = img.clone();
    let region = PsdRect {
        top: 12,
        left: 12,
        bottom: 20,
        right: 20,
    };
    let done = clone_region(&mut img, region, &all_ones(region), (0, 0), Transfer::Full);
    assert_eq!(done, None);
    assert_eq!(img, before);
}

#[test]
fn cloning_carries_the_source_texture_over() {
    // Left half dark, right half light. Clone a light patch (offset +8 in x)
    // into a hole on the dark side: the solve keeps the destination's low
    // level and takes the source's texture.
    let (w, h) = (48, 48);
    let mut img = flat(w, h, [30, 30, 30, 255]);
    for y in 0..h {
        for x in 24..w {
            let v = 180 + ((x - 24) as u8 % 20);
            img.set(x, y, [v, v, v, 255]);
        }
    }
    let region = PsdRect {
        top: 20,
        left: 8,
        bottom: 28,
        right: 16,
    };
    clone_region(&mut img, region, &all_ones(region), (16, 0), Transfer::Full);
    // The transplanted patch is near the destination's level (dark), not the
    // source's absolute level, because the boundary is dark.
    let v = px(&img, 12, 24)[0] as i32;
    assert!(v < 120, "clone ignored the destination's level: {v}");
}

#[test]
fn texture_only_clone_keeps_destination_colour() {
    let (w, h) = (48, 48);
    let mut img = flat(w, h, [40, 60, 80, 255]);
    // Source region (offset +16): a saturated red patch.
    for y in 0..h {
        for x in 32..40 {
            img.set(x, y, [255, 0, 0, 255]);
        }
    }
    let region = PsdRect {
        top: 20,
        left: 16,
        bottom: 28,
        right: 24,
    };
    clone_region(
        &mut img,
        region,
        &all_ones(region),
        (16, 0),
        Transfer::TextureOnly,
    );
    // Hue stays with the destination: red does not dominate.
    let p = px(&img, 20, 24);
    assert!(
        p[0] as i32 - (p[2] as i32) < 60,
        "TextureOnly leaked the source colour: {p:?}"
    );
}

#[test]
fn mode_round_trips_through_its_integer() {
    assert_eq!(HealMode::from_i32(0), Some(HealMode::ProximityMatch));
    assert_eq!(HealMode::from_i32(1), Some(HealMode::CreateTexture));
    assert_eq!(HealMode::from_i32(2), Some(HealMode::ContentAware));
    assert_eq!(HealMode::from_i32(3), None);
    assert_eq!(HealMode::default(), HealMode::ProximityMatch);
}

fn layer_doc(w: u32, h: u32, rgba: [u8; 4]) -> Document {
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let n = (w * h) as usize;
    {
        let _ = doc.layers.push(Layer {
            name: "px".into(),
            rect: rect(w as i32, h as i32),
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            color: ColorLabel::None,
            clipping: false,
            visible: true,
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![rgba[0]; n].into(),
                },
                Channel {
                    id: 1,
                    data: vec![rgba[1]; n].into(),
                },
                Channel {
                    id: 2,
                    data: vec![rgba[2]; n].into(),
                },
                Channel {
                    id: -1,
                    data: vec![rgba[3]; n].into(),
                },
            ],
            ..Layer::default()
        });
    };
    doc
}

fn channel(doc: &Document, layer: usize, id: i16, x: i32, y: i32) -> u8 {
    let l = &doc.layers[layer];
    let i = (y * l.rect.width() + x) as usize;
    l.channels.iter().find(|c| c.id == id).unwrap().data[i]
}

#[test]
fn heal_layer_writes_the_pixels_and_returns_a_document_rect() {
    let mut doc = layer_doc(32, 32, BASE);
    // A blemish on a flat field.
    {
        let l = &mut doc.layers[0];
        for y in 12..20 {
            for x in 12..20 {
                let i = (y * 32 + x) as usize;
                for c in &mut l.channels[..3] {
                    c.data[i] = 255;
                }
            }
        }
    }
    let region = PsdRect {
        top: 12,
        left: 12,
        bottom: 20,
        right: 20,
    };
    let done = heal_layer(
        &mut doc,
        "0",
        region,
        &all_ones(region),
        |img, local, cov| heal_region(img, local, cov, HealMode::ProximityMatch),
    )
    .unwrap();
    assert_eq!(done, Some(region));
    assert_eq!(channel(&doc, 0, 0, 16, 16), BASE[0]);
    assert_eq!(channel(&doc, 0, 1, 16, 16), BASE[1]);
    assert_eq!(channel(&doc, 0, 2, 16, 16), BASE[2]);
}

#[test]
fn heal_layer_refuses_a_group_or_bad_path() {
    let mut doc = layer_doc(16, 16, BASE);
    {
        let _ = doc.layers.push(Layer {
            name: "group".into(),
            is_group: true,
            ..Layer::default()
        });
    };
    let region = rect(4, 4);
    let op = |img: &mut RgbaImage, local, cov: &[f32]| {
        heal_region(img, local, cov, HealMode::ProximityMatch)
    };
    assert_eq!(
        heal_layer(&mut doc, "9", region, &all_ones(region), op),
        Err(HealError::NoRasterLayer)
    );
    assert_eq!(
        heal_layer(&mut doc, "nope", region, &all_ones(region), op),
        Err(HealError::NoRasterLayer)
    );
    assert_eq!(
        heal_layer(&mut doc, "1", region, &all_ones(region), op),
        Err(HealError::NoRasterLayer)
    );
}

#[test]
fn heal_layer_refuses_a_pixel_locked_layer() {
    let mut doc = layer_doc(16, 16, BASE);
    doc.layers[0].lock = doc.layers[0].lock.with(LockFlags::PIXELS, true);
    let region = rect(4, 4);
    assert_eq!(
        heal_layer(
            &mut doc,
            "0",
            region,
            &all_ones(region),
            |img, local, cov| { heal_region(img, local, cov, HealMode::ProximityMatch) }
        ),
        Err(HealError::Locked)
    );
}

#[test]
fn a_transparency_lock_keeps_alpha() {
    let mut doc = layer_doc(16, 16, [BASE[0], BASE[1], BASE[2], 40]);
    doc.layers[0].lock = doc.layers[0].lock.with(LockFlags::TRANSPARENCY, true);
    let region = rect(4, 4);
    let done = heal_layer(
        &mut doc,
        "0",
        region,
        &all_ones(region),
        |img, local, cov| {
            // Pretend the heal changed alpha; the wrapper must restore it.
            let r = heal_region(img, local, cov, HealMode::ProximityMatch);
            img.data.iter_mut().for_each(|p| p[3] = 0);
            r
        },
    )
    .unwrap();
    assert!(done.is_some());
    assert_eq!(channel(&doc, 0, -1, 8, 8), 40, "alpha was not preserved");
}

#[test]
fn a_spot_gesture_heals_under_its_dab() {
    let mut doc = layer_doc(32, 32, BASE);
    // A white blemish the brush will cover.
    for y in 14..18 {
        for x in 14..18 {
            let i = (y * 32 + x) as usize;
            for c in &mut doc.layers[0].channels[..3] {
                c.data[i] = 255;
            }
        }
    }
    let mut stroke = HealStroke::begin(&doc, "0", 8, 100).unwrap();
    assert_eq!(stroke.layer_rect(), rect(32, 32));
    assert!(stroke.dab(16.0, 16.0).is_some());
    assert!(stroke.is_painted());
    let outcome = stroke
        .commit(&mut doc, HealMode::ProximityMatch, None, Transfer::Full)
        .unwrap()
        .expect("a populated mask commits");
    assert_eq!(
        outcome.document.layers[0].channels[0].data[(16 * 32 + 16) as usize],
        BASE[0]
    );
}

#[test]
fn an_empty_gesture_commits_nothing() {
    let mut doc = layer_doc(16, 16, BASE);
    let stroke = HealStroke::begin(&doc, "0", 8, 100).unwrap();
    assert!(!stroke.is_painted());
    assert!(stroke
        .commit(&mut doc, HealMode::ProximityMatch, None, Transfer::Full)
        .unwrap()
        .is_none());
}

#[test]
fn a_clone_gesture_uses_its_source_offset() {
    let (w, h) = (48, 48);
    let mut doc = layer_doc(w, h, [30, 30, 30, 255]);
    // Light band on the right for the source offset to sample.
    for y in 0..h {
        for x in 24..w {
            let i = (y * w + x) as usize;
            for c in &mut doc.layers[0].channels[..3] {
                c.data[i] = 180;
            }
        }
    }
    let mut stroke = HealStroke::begin(&doc, "0", 8, 100).unwrap();
    stroke.dab(12.0, 24.0);
    let outcome = stroke
        .commit(
            &mut doc,
            HealMode::ProximityMatch,
            Some((16, 0)),
            Transfer::TextureOnly,
        )
        .unwrap()
        .expect("a populated mask commits");
    // TextureOnly keeps the destination's dark level, not the source's light.
    let v = outcome.document.layers[0].channels[0].data[(24 * w + 12) as usize];
    assert!(v < 120, "clone ignored the destination's level: {v}");
}

/// A light 140×60 field with a dark 20×20 blot at (20, 20), and a selection
/// mask covering `sel` (x, y, w, h). Adapted from photorust's Patch tests.
fn patch_doc(field: [u8; 4], sel: (i32, i32, i32, i32)) -> (Document, Vec<u8>) {
    let mut doc = layer_doc(140, 60, field);
    paint(&mut doc, (20, 20, 20, 20), [40, 30, 30]);
    let mut mask = vec![0u8; 140 * 60];
    for y in sel.1..sel.1 + sel.3 {
        for x in sel.0..sel.0 + sel.2 {
            mask[(y * 140 + x) as usize] = 255;
        }
    }
    (doc, mask)
}

fn paint(doc: &mut Document, (x0, y0, w, h): (i32, i32, i32, i32), rgb: [u8; 3]) {
    let l = &mut doc.layers[0];
    let lw = l.rect.width();
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            for (c, v) in l.channels[..3].iter_mut().zip(rgb) {
                c.data[(y * lw + x) as usize] = v;
            }
        }
    }
}

const PATCH_FIELD: [u8; 4] = [210, 200, 190, 255];

#[test]
fn patch_in_source_mode_repairs_the_selection() {
    let (mut doc, mask) = patch_doc(PATCH_FIELD, (20, 20, 20, 20));
    let options = PatchOptions {
        dx: 60,
        ..PatchOptions::default()
    };
    let done = patch_layer(&mut doc, "0", &mask, options).unwrap();
    assert!(done.is_some());
    assert!(
        channel(&doc, 0, 0, 30, 30) > 140,
        "the blot survived the patch"
    );
    assert_eq!(
        channel(&doc, 0, 0, 90, 30),
        PATCH_FIELD[0],
        "the source was modified"
    );
}

#[test]
fn source_and_destination_modes_change_opposite_ends() {
    let drag = PatchOptions {
        dx: 60,
        ..PatchOptions::default()
    };
    let (mut source, mask) = patch_doc(PATCH_FIELD, (20, 20, 20, 20));
    patch_layer(&mut source, "0", &mask, drag).unwrap();
    let (mut destination, mask) = patch_doc(PATCH_FIELD, (20, 20, 20, 20));
    let dest = PatchOptions {
        destination: true,
        ..drag
    };
    let done = patch_layer(&mut destination, "0", &mask, dest).unwrap();
    // Destination edits the drag target, 60 px right of the selection.
    assert_eq!(
        done,
        Some(PsdRect {
            top: 20,
            left: 80,
            bottom: 40,
            right: 100
        })
    );
    assert!(channel(&source, 0, 0, 30, 30) > 140);
    assert!(
        channel(&destination, 0, 0, 30, 30) < 120,
        "the selection changed"
    );
    assert!(
        channel(&destination, 0, 0, 90, 30) < 160,
        "the target was not patched"
    );
}

#[test]
fn transparent_patch_keeps_the_destination_colour() {
    // A blue field patched from a red area 70 px right: blue must survive.
    let (_, mask) = patch_doc(PATCH_FIELD, (20, 20, 20, 20));
    let mut doc = layer_doc(140, 60, [60, 90, 200, 255]);
    paint(&mut doc, (80, 20, 40, 20), [200, 80, 40]);
    let options = PatchOptions {
        dx: 70,
        transparent: true,
        ..PatchOptions::default()
    };
    patch_layer(&mut doc, "0", &mask, options).unwrap();
    assert!(channel(&doc, 0, 2, 30, 30) > channel(&doc, 0, 0, 30, 30));
}

#[test]
fn content_aware_patch_ignores_the_drag() {
    let run = |dx, dy| {
        let (mut doc, mask) = patch_doc(PATCH_FIELD, (20, 20, 20, 20));
        let options = PatchOptions {
            dx,
            dy,
            content_aware: true,
            ..PatchOptions::default()
        };
        assert!(patch_layer(&mut doc, "0", &mask, options)
            .unwrap()
            .is_some());
        doc.layers[0].channels.clone()
    };
    let a = run(60, 0);
    assert_eq!(a, run(-10, 25));
}

#[test]
fn patch_refuses_an_empty_selection_a_zero_drag_or_a_locked_layer() {
    let (mut doc, mask) = patch_doc(PATCH_FIELD, (20, 20, 20, 20));
    let drag = PatchOptions {
        dx: 60,
        ..PatchOptions::default()
    };
    let empty = vec![0u8; mask.len()];
    assert_eq!(patch_layer(&mut doc, "0", &empty, drag), Ok(None));
    assert_eq!(patch_layer(&mut doc, "0", &mask[1..], drag), Ok(None));
    assert_eq!(
        patch_layer(&mut doc, "0", &mask, PatchOptions::default()),
        Ok(None)
    );
    doc.layers[0].lock = doc.layers[0].lock.with(LockFlags::PIXELS, true);
    assert_eq!(
        patch_layer(&mut doc, "0", &mask, drag),
        Err(HealError::Locked)
    );
}

/// A light 96×48 field with a dark 12×12 blob at (18, 18), and a mask over the
/// 16×16 box around it. Adapted from photorust's `move_region` tests.
fn move_doc() -> (Document, Vec<u8>) {
    let mut doc = layer_doc(96, 48, PATCH_FIELD);
    paint(&mut doc, (18, 18, 12, 12), [50, 40, 40]);
    let mut mask = vec![0u8; 96 * 48];
    for y in 16..32 {
        for x in 16..32 {
            mask[y * 96 + x] = 255;
        }
    }
    (doc, mask)
}

fn drag(dx: i32, extend: bool) -> MoveOptions {
    MoveOptions {
        dx,
        extend,
        ..MoveOptions::default()
    }
}

#[test]
fn a_move_relocates_the_pixels_and_fills_the_hole() {
    let (mut doc, mask) = move_doc();
    let done = move_layer(&mut doc, "0", &mask, drag(40, false)).unwrap();
    // The hole and the destination, 40 px right.
    assert_eq!(
        done,
        Some(PsdRect {
            top: 16,
            left: 16,
            bottom: 32,
            right: 72
        })
    );
    // Copied verbatim, not re-solved.
    assert_eq!(channel(&doc, 0, 0, 64, 24), 50);
    let vacated = channel(&doc, 0, 0, 24, 24);
    assert!(
        vacated.abs_diff(PATCH_FIELD[0]) <= 20,
        "hole left: {vacated}"
    );
}

#[test]
fn extend_mode_leaves_the_original_in_place() {
    let (mut doc, mask) = move_doc();
    move_layer(&mut doc, "0", &mask, drag(40, true)).unwrap();
    assert_eq!(
        channel(&doc, 0, 0, 24, 24),
        50,
        "extend erased the original"
    );
    assert_eq!(channel(&doc, 0, 0, 64, 24), 50, "extend did not copy");
}

#[test]
fn every_adaptation_fills_the_hole_deterministically() {
    for level in 0..5 {
        let adaptation = Adaptation::from_i32(level).unwrap();
        let run = || {
            let (mut doc, mask) = move_doc();
            let options = MoveOptions {
                adaptation,
                ..drag(40, false)
            };
            move_layer(&mut doc, "0", &mask, options).unwrap();
            doc.layers[0].channels.clone()
        };
        let a = run();
        assert_eq!(a, run(), "{adaptation:?} is not deterministic");
        let vacated = a[0].data[24 * 96 + 24];
        assert!(
            vacated.abs_diff(PATCH_FIELD[0]) <= 20,
            "{adaptation:?}: {vacated}"
        );
    }
    assert_eq!(Adaptation::from_i32(5), None);
    assert_eq!(Adaptation::default(), Adaptation::Medium);
}

#[test]
fn a_move_refuses_a_zero_drag_an_off_layer_target_or_a_locked_layer() {
    let (mut doc, mask) = move_doc();
    assert_eq!(move_layer(&mut doc, "0", &mask, drag(0, false)), Ok(None));
    assert_eq!(move_layer(&mut doc, "0", &mask, drag(500, false)), Ok(None));
    assert_eq!(
        move_layer(&mut doc, "0", &mask[1..], drag(40, false)),
        Ok(None)
    );
    doc.layers[0].lock = doc.layers[0].lock.with(LockFlags::PIXELS, true);
    assert_eq!(
        move_layer(&mut doc, "0", &mask, drag(40, false)),
        Err(HealError::Locked)
    );
}

const SKIN: [u8; 4] = [215, 175, 150, 255];

fn red_eye_doc(pupil: [u8; 3]) -> Document {
    let mut doc = layer_doc(40, 40, SKIN);
    paint(&mut doc, (18, 18, 5, 5), pupil);
    doc
}

fn eye_box() -> PsdRect {
    PsdRect {
        top: 14,
        left: 14,
        bottom: 26,
        right: 26,
    }
}

#[test]
fn red_eye_neutralises_the_pupil_and_leaves_skin_alone() {
    let mut doc = red_eye_doc([220, 30, 30]);
    let dirty = red_eye_layer(&mut doc, "0", eye_box(), 50, 50).unwrap();
    assert_eq!(dirty, Some(eye_box()));
    assert!(channel(&doc, 0, 0, 20, 20) < 80, "the pupil is still red");
    assert_eq!(channel(&doc, 0, 0, 15, 15), SKIN[0], "the skin was drained");
    assert_eq!(channel(&doc, 0, 1, 15, 15), SKIN[1], "the skin was drained");
}

#[test]
fn darken_amount_controls_how_dark_the_pupil_ends_up() {
    let mut light = red_eye_doc([220, 40, 40]);
    let mut dark = red_eye_doc([220, 40, 40]);
    red_eye_layer(&mut light, "0", eye_box(), 50, 0).unwrap();
    red_eye_layer(&mut dark, "0", eye_box(), 50, 100).unwrap();
    assert!(channel(&dark, 0, 0, 20, 20) < channel(&light, 0, 0, 20, 20));
}

#[test]
fn pupil_size_widens_what_counts_as_red() {
    // Red leads, but only by 1.3x: too weak at Pupil Size 0, caught at 100.
    let mut tight = red_eye_doc([130, 100, 100]);
    let mut wide = red_eye_doc([130, 100, 100]);
    assert_eq!(
        red_eye_layer(&mut tight, "0", eye_box(), 0, 50).unwrap(),
        None
    );
    assert!(red_eye_layer(&mut wide, "0", eye_box(), 100, 50)
        .unwrap()
        .is_some());
}

#[test]
fn red_eye_ignores_no_red_an_off_canvas_box_or_a_locked_layer() {
    let mut doc = layer_doc(32, 32, [80, 140, 200, 255]);
    let before = doc.clone();
    assert_eq!(
        red_eye_layer(&mut doc, "0", eye_box(), 50, 50).unwrap(),
        None
    );
    let off = PsdRect {
        top: 40,
        left: 40,
        bottom: 50,
        right: 50,
    };
    assert_eq!(red_eye_layer(&mut doc, "0", off, 50, 50).unwrap(), None);
    assert_eq!(doc.layers[0].channels, before.layers[0].channels);

    let mut locked = red_eye_doc([220, 30, 30]);
    locked.layers[0].lock = locked.layers[0].lock.with(LockFlags::PIXELS, true);
    assert_eq!(
        red_eye_layer(&mut locked, "0", eye_box(), 50, 50),
        Err(HealError::Locked)
    );
}
