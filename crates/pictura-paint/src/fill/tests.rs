use super::*;
use crate::stroke::layer_rgba;
use pictura_core::{Channel, Layer, LockFlags};

pub(crate) const RED: Rgba = Rgba {
    r: 220,
    g: 0,
    b: 0,
    a: 255,
};

/// A `w × h` document with one layer of `rgba` at `(left, top)`; a
/// Background (no alpha channel) when `background`.
pub(crate) fn doc_at(
    (w, h): (i32, i32),
    (left, top): (i32, i32),
    rgba: [u8; 4],
    background: bool,
) -> Document {
    let mut doc = Document::new(w as u32, h as u32, ColorMode::Rgb, BitDepth::Eight);
    let ids: &[i16] = if background {
        &[0, 1, 2]
    } else {
        &[0, 1, 2, -1]
    };
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        rect: PsdRect {
            top,
            left,
            bottom: top + h,
            right: left + w,
        },
        channels: ids
            .iter()
            .map(|&id| Channel {
                id,
                data: vec![rgba[if id < 0 { 3 } else { id as usize }]; (w * h) as usize].into(),
            })
            .collect(),
        background,
        ..Default::default()
    }]);
    doc
}

pub(crate) fn doc(w: i32, h: i32, rgba: [u8; 4]) -> Document {
    doc_at((w, h), (0, 0), rgba, false)
}

/// Layer-local pixel `(x, y)` of the first layer.
pub(crate) fn pixel(doc: &Document, x: i32, y: i32) -> [u8; 4] {
    layer_rgba(&doc.layers[0]).get(x, y)
}

pub(crate) fn lock(doc: &mut Document, flag: u8) {
    doc.layers[0].lock = LockFlags::default().with(flag, true);
}

fn flood(doc: &mut Document, mode: PaintMode, selection: Option<&[u8]>) -> Option<PsdRect> {
    fill_layer(doc, "0", mode, selection, Alpha::Blend, |_, _| {
        Some((RED, 1.0))
    })
}

#[test]
fn a_fill_covers_the_layer_and_reports_what_changed() {
    let mut d = doc(8, 4, [255; 4]);
    let dirty = flood(&mut d, PaintMode::Normal, None).expect("filled");
    assert_eq!(
        (dirty.left, dirty.top, dirty.right, dirty.bottom),
        (0, 0, 8, 4)
    );
    assert_eq!(pixel(&d, 7, 3), [220, 0, 0, 255]);
    assert!(
        flood(&mut d, PaintMode::Normal, None).is_none(),
        "an unchanged layer reported a change"
    );
}

#[test]
fn the_selection_confines_and_scales_the_fill() {
    let mut d = doc(4, 1, [255; 4]);
    let dirty = flood(&mut d, PaintMode::Normal, Some(&[255, 128, 0, 0])).expect("filled");
    assert_eq!((dirty.left, dirty.right), (0, 2));
    assert_eq!(pixel(&d, 0, 0), [220, 0, 0, 255]);
    let half = pixel(&d, 1, 0);
    assert!((half[1] as i32 - 127).abs() <= 1, "green {}", half[1]);
    assert_eq!(pixel(&d, 2, 0), [255; 4]);
    assert!(
        flood(&mut d, PaintMode::Normal, Some(&[255])).is_none(),
        "a short selection was read"
    );
}

#[test]
fn a_layer_is_filled_in_document_space() {
    // The layer sits at (3, 0) and hangs off the 8 px canvas.
    let mut d = doc_at((8, 2), (3, 0), [255; 4], false);
    let asked = std::sync::Mutex::new(Vec::new());
    let dirty = fill_layer(
        &mut d,
        "0",
        PaintMode::Normal,
        None,
        Alpha::Blend,
        |x, y| {
            asked.lock().unwrap().push((x, y));
            (x == 5).then_some((RED, 1.0))
        },
    )
    .expect("filled");
    let asked = asked.into_inner().unwrap();
    assert_eq!((dirty.left, dirty.right), (5, 6));
    assert_eq!(pixel(&d, 2, 1), [220, 0, 0, 255]);
    assert!(asked.iter().all(|&(x, _)| (3..8).contains(&x)));
}

#[test]
fn a_transparency_lock_keeps_alpha_and_refuses_clear() {
    let mut d = doc(4, 1, [10, 10, 10, 0]);
    d.layers[0].channels[3].data[0] = 200;
    lock(&mut d, LockFlags::TRANSPARENCY);
    flood(&mut d, PaintMode::Normal, None).expect("filled");
    assert_eq!(pixel(&d, 0, 0), [220, 0, 0, 200]);
    assert_eq!(
        pixel(&d, 1, 0),
        [10, 10, 10, 0],
        "an empty pixel gained colour"
    );
    assert!(flood(&mut d, PaintMode::Clear, None).is_none());
}

#[test]
fn a_keep_policy_keeps_the_destination_alpha() {
    // An opaque pixel, a half transparent one and an empty one.
    let mut d = doc(3, 1, [255; 4]);
    d.layers[0].channels[3].data[1] = 128;
    d.layers[0].channels[3].data[2] = 0;
    fill_layer(&mut d, "0", PaintMode::Normal, None, Alpha::Keep, |_, _| {
        Some((RED, 1.0))
    })
    .expect("filled");
    assert_eq!(
        pixel(&d, 0, 0),
        [220, 0, 0, 255],
        "the opaque pixel took the colour"
    );
    assert_eq!(
        pixel(&d, 1, 0),
        [220, 0, 0, 128],
        "the half transparent pixel kept its alpha"
    );
    assert_eq!(
        pixel(&d, 2, 0),
        [255, 255, 255, 0],
        "the empty pixel was filled"
    );
}

#[test]
fn a_replace_policy_takes_the_sources_alpha() {
    let mut d = doc(2, 1, [255; 4]);
    let see_through = Rgba {
        r: 220,
        g: 0,
        b: 0,
        a: 0,
    };
    fill_layer(
        &mut d,
        "0",
        PaintMode::Normal,
        None,
        Alpha::Replace,
        |_, _| Some((see_through, 1.0)),
    )
    .expect("filled");
    assert_eq!(pixel(&d, 0, 0)[3], 0, "the source's alpha is the pixel's");
}

#[test]
fn a_transparency_lock_outranks_the_policy() {
    let mut d = doc(2, 1, [255; 4]);
    lock(&mut d, LockFlags::TRANSPARENCY);
    let see_through = Rgba {
        r: 220,
        g: 0,
        b: 0,
        a: 0,
    };
    fill_layer(
        &mut d,
        "0",
        PaintMode::Normal,
        None,
        Alpha::Replace,
        |_, _| Some((see_through, 1.0)),
    );
    assert_eq!(pixel(&d, 0, 0)[3], 255, "the lock kept the alpha");
}

#[test]
fn locked_pixels_a_group_and_other_modes_are_refused() {
    let mut locked = doc(4, 1, [255; 4]);
    lock(&mut locked, LockFlags::PIXELS);
    assert!(flood(&mut locked, PaintMode::Normal, None).is_none());

    let mut group = doc(4, 1, [255; 4]);
    group.layers[0].is_group = true;
    assert!(flood(&mut group, PaintMode::Normal, None).is_none());

    let mut deep = doc(4, 1, [255; 4]);
    deep.depth = BitDepth::Sixteen;
    assert!(flood(&mut deep, PaintMode::Normal, None).is_none());
}

#[test]
fn behind_fills_only_what_is_not_opaque_and_clear_erases() {
    let mut d = doc(2, 1, [255; 4]);
    d.layers[0].channels[3].data[1] = 0;
    flood(&mut d, PaintMode::Behind, None).expect("filled");
    assert_eq!(pixel(&d, 0, 0), [255; 4]);
    assert_eq!(pixel(&d, 1, 0), [220, 0, 0, 255]);

    flood(&mut d, PaintMode::Clear, None).expect("cleared");
    assert_eq!(pixel(&d, 0, 0)[3], 0);

    let mut background = doc_at((2, 1), (0, 0), [255; 4], true);
    assert!(
        flood(&mut background, PaintMode::Clear, None).is_none(),
        "a layer without alpha was cleared"
    );
}

#[test]
fn dissolve_paints_whole_pixels_in_proportion_and_repeats() {
    let run = || {
        let mut d = doc(64, 64, [255; 4]);
        let see_through = Rgba { a: 128, ..RED };
        fill_layer(
            &mut d,
            "0",
            PaintMode::Dissolve,
            None,
            Alpha::Blend,
            |_, _| Some((see_through, 0.5)),
        );
        d
    };
    let d = run();
    let painted = layer_rgba(&d.layers[0])
        .data
        .iter()
        .inspect(|px| assert!(**px == [220, 0, 0, 255] || **px == [255; 4]))
        .filter(|px| **px != [255; 4])
        .count();
    // Half strength of a half-opaque colour: about a quarter of 4096.
    assert!((800..1250).contains(&painted), "{painted} pixels");
    assert_eq!(layer_rgba(&run().layers[0]), layer_rgba(&d.layers[0]));
}

/// A Grayscale document's layer carries one colour plane.
pub(crate) fn gray_doc(w: i32, h: i32, rgba: [u8; 4]) -> Document {
    let mut doc = Document::new(w as u32, h as u32, ColorMode::Grayscale, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: h,
            right: w,
        },
        channels: vec![
            Channel {
                id: 0,
                data: vec![rgba[0]; (w * h) as usize].into(),
            },
            Channel {
                id: -1,
                data: vec![rgba[3]; (w * h) as usize].into(),
            },
        ],
        ..Default::default()
    }]);
    doc
}

#[test]
fn a_gray_layer_takes_the_colours_luma() {
    let mut d = gray_doc(2, 1, [255, 255, 255, 255]);
    flood(&mut d, PaintMode::Normal, None);
    // 0.299 * 220 = 65.78, the Rec. 601 luma of #DC0000.
    assert_eq!(d.layers[0].channels[0].data.as_slice(), &[66, 66]);
    let alpha = d.layers[0]
        .channels
        .iter()
        .find(|c| c.id == -1)
        .expect("an alpha channel");
    assert_eq!(alpha.data.as_slice(), &[255, 255]);
}

#[test]
fn an_rgb_layer_keeps_the_colours_channels() {
    let mut d = doc(2, 1, [255; 4]);
    flood(&mut d, PaintMode::Normal, None);
    assert_eq!(d.layers[0].channels[0].data.as_slice(), &[220, 220]);
    assert_eq!(d.layers[0].channels[1].data.as_slice(), &[0, 0]);
    assert_eq!(d.layers[0].channels[2].data.as_slice(), &[0, 0]);
}

#[test]
fn a_layer_above_the_canvas_fills_the_rows_on_the_canvas() {
    // A layer that hangs above the canvas: its first rows are off-canvas,
    // and every plane has to be cut on the same span.
    let mut d = doc_at((4, 4), (0, -2), [255, 255, 255, 255], false);
    for (i, byte) in d.layers[0].channels[3]
        .data
        .as_mut_slice()
        .iter_mut()
        .enumerate()
    {
        *byte = (i * 10) as u8;
    }
    fill_layer(
        &mut d,
        "0",
        PaintMode::Normal,
        None,
        Alpha::Blend,
        |_, _| Some((RED, 1.0)),
    );
    // The canvas rows are the layer's last two rows, and their alpha took
    // the fill; the rows above the canvas are untouched.
    let alpha = d.layers[0].channels[3].data.as_slice();
    assert_eq!(&alpha[8..12], &[255; 4]);
    assert_eq!(&alpha[0..8], &[0, 10, 20, 30, 40, 50, 60, 70]);
}

#[test]
fn an_eight_bit_fill_is_byte_identical() {
    // A Normal fill of #DC0000 at full coverage over a white layer, with
    // the selection at half strength on the second pixel.
    let mut d = doc(2, 1, [255; 4]);
    fill_layer(
        &mut d,
        "0",
        PaintMode::Normal,
        Some(&[255, 128]),
        Alpha::Blend,
        |_, _| Some((RED, 1.0)),
    );
    assert_eq!(d.layers[0].channels[0].data.as_slice(), &[220, 237]);
    assert_eq!(d.layers[0].channels[1].data.as_slice(), &[0, 127]);
    assert_eq!(d.layers[0].channels[2].data.as_slice(), &[0, 127]);
    assert_eq!(d.layers[0].channels[3].data.as_slice(), &[255, 255]);
}

#[test]
fn grain_noise_is_deterministic_and_clumpy() {
    let mut values = Vec::new();
    let mut blocks = Vec::new();
    for y in 0..64 {
        for x in 0..64 {
            let v = grain_noise(x, y);
            assert!((0.0..=1.0).contains(&v), "grain {v} out of range");
            assert_eq!(v, grain_noise(x, y), "grain is not deterministic");
            values.push(v);
        }
    }
    for y in 0..32 {
        for x in 0..32 {
            let (x, y) = (2 * x, 2 * y);
            blocks.push(
                (grain_noise(x, y)
                    + grain_noise(x + 1, y)
                    + grain_noise(x, y + 1)
                    + grain_noise(x + 1, y + 1))
                    / 4.0,
            );
        }
    }
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
    let std = |v: &[f32]| {
        let m = mean(v);
        (v.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / v.len() as f32).sqrt()
    };
    // A 2 px box average of pure per-pixel static loses half its spread;
    // clumped grain keeps most of it.
    assert!(
        std(&blocks) > std(&values) * 0.6,
        "grain is not clumped: block spread {:.3} vs pixel spread {:.3}",
        std(&blocks),
        std(&values)
    );
    let neighbour = {
        let m = mean(&values);
        let pairs = (0..64).flat_map(|y| (0..63).map(move |x| (y * 64 + x, y * 64 + x + 1)));
        let cov: f32 = pairs
            .map(|(a, b)| (values[a] - m) * (values[b] - m))
            .sum::<f32>()
            / (64.0 * 63.0);
        cov / (std(&values) * std(&values))
    };
    // The 2B Pencil reference's clumps: neighbours correlate more than the
    // earlier static-heavy mix (~0.47) without smearing into blotches.
    assert!(
        (0.55..=0.72).contains(&neighbour),
        "grain neighbour correlation {neighbour:.3}"
    );
}

#[test]
fn a_run_of_equal_pixels_blends_like_each_pixel_alone() {
    // Runs of equal backdrops and strengths, broken by changes of each
    // input, so the reused blend is checked against a fresh one.
    let (w, h) = (24, 3);
    let mut d = doc(w, h, [0; 4]);
    for (i, v) in d.layers[0].channels[3].data.iter_mut().enumerate() {
        *v = [0, 0, 0, 90, 90, 255][i % 6];
    }
    for (i, v) in d.layers[0].channels[0].data.iter_mut().enumerate() {
        *v = if i % 12 < 6 { 10 } else { 200 };
    }
    let before = layer_rgba(&d.layers[0]);
    let strength = |x: i32| [1.0, 1.0, 0.5, 0.5, 0.5, 0.25][(x % 6) as usize];
    for mode in [PaintMode::Normal, PaintMode::Multiply, PaintMode::Behind] {
        let mut filled = d.clone();
        fill_layer(&mut filled, "0", mode, None, Alpha::Blend, |x, _| {
            Some((RED, strength(x)))
        });
        for y in 0..h {
            for x in 0..w {
                let [r, g, b, a] = before.get(x, y);
                let want = blend_pixel(mode, (r, g, b, a), &RED, strength(x), 0.0)
                    .map_or([r, g, b, a], |(r, g, b, a)| [r, g, b, a]);
                assert_eq!(pixel(&filled, x, y), want, "{mode:?} at ({x}, {y})");
            }
        }
    }
}
