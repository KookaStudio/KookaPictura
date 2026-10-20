//! Profiling harness for the Move-tool selection drag path (#754).
//!
//! Times the three pieces the drag and release spend their time in, on the
//! issue's case (4000×4000 document, 2000×2000 rectangular selection):
//!
//! * `move_mask_selection_content` — the release stall on the mask target;
//! * `Selection::translate` — the per-move plane materialisation;
//! * the selection-bounds scan and `crop_selection` — app-side loops that are
//!   private, mirrored here line for line so the harness can time them.
//!
//! Run it in the app's profile and in an optimised one to see the opt-level
//! factor:
//!
//! ```text
//! cargo test -p pictura-render --test selection_move_perf -- --ignored --nocapture
//! cargo test -p pictura-render --test selection_move_perf --release -- --ignored --nocapture
//! ```
//!
//! The first is what a CMake `Debug` build links (Corrosion maps it to the dev
//! profile, opt-level 0); the second is the ceiling the profile override in
//! `Cargo.toml` approaches.

use std::time::{Duration, Instant};

use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LayerMask, LockFlags,
    PsdRect,
};
use pictura_render::move_mask_selection_content;
use pictura_select::Selection;

const W: u32 = 4000;
const H: u32 = 4000;
const SEL_X: u32 = 1000;
const SEL_Y: u32 = 1000;
const SEL_W: u32 = 2000;
const SEL_H: u32 = 2000;

fn plane(value: u8, n: usize) -> Vec<u8> {
    vec![value; n]
}

fn pixel_layer(w: u32, h: u32, value: u8) -> Layer {
    let n = w as usize * h as usize;
    Layer {
        name: "base".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: plane(value, n).into(),
            },
            Channel {
                id: 1,
                data: plane(value, n).into(),
            },
            Channel {
                id: 2,
                data: plane(value, n).into(),
            },
            Channel {
                id: -1,
                data: plane(255, n).into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: true,
        ..Default::default()
    }
}

fn masked_document() -> Document {
    let mut doc = Document::new(W, H, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel_layer(W, H, 128);
    layer.mask = Some(LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: H as i32,
            right: W as i32,
        },
        default_color: 255,
        disabled: false,
        flags: 0,
        data: Some(plane(128, (W * H) as usize).into()),
        extra: Vec::new(),
        ..Default::default()
    });
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc
}

fn rect_selection() -> Selection {
    let mut sel = Selection::none(W, H);
    for y in SEL_Y..SEL_Y + SEL_H {
        let row = y as usize * W as usize;
        sel.data[row + SEL_X as usize..row + (SEL_X + SEL_W) as usize].fill(255);
    }
    sel
}

/// `impl_selection.rs`'s bounds path, now `Selection::bounds`.
fn selection_bounds_extents(sel: &Selection) -> Vec<i32> {
    let Some(bounds) = sel.bounds() else {
        return Vec::new();
    };
    vec![
        bounds.left,
        bounds.top,
        bounds.right - bounds.left,
        bounds.bottom - bounds.top,
    ]
}

/// `helpers_composite.rs::crop_selection`, verbatim.
fn crop_selection(data: &[u8], doc_w: u32, doc_h: u32, rect: PsdRect) -> Vec<u8> {
    let (w, h) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
    let mut out = vec![0u8; w * h];
    if w == 0 || h == 0 {
        return out;
    }
    let x0 = rect.left.max(0);
    let x1 = rect.right.min(doc_w as i32);
    if x1 <= x0 {
        return out;
    }
    let (col0, cols) = ((x0 - rect.left) as usize, (x1 - x0) as usize);
    for y in 0..h {
        let dy = rect.top + y as i32;
        if dy < 0 || dy >= doc_h as i32 {
            continue;
        }
        let src = dy as usize * doc_w as usize + x0 as usize;
        let dst = y * w + col0;
        out[dst..dst + cols].copy_from_slice(&data[src..src + cols]);
    }
    out
}

fn report(name: &str, samples: &mut [Duration]) {
    samples.sort();
    let min = samples[0];
    let median = samples[samples.len() / 2];
    let mean = samples.iter().sum::<Duration>() / samples.len() as u32;
    println!(
        "{name:<34} min {:>8.2} ms  median {:>8.2} ms  mean {:>8.2} ms  (n={})",
        min.as_secs_f64() * 1e3,
        median.as_secs_f64() * 1e3,
        mean.as_secs_f64() * 1e3,
        samples.len()
    );
}

fn time<T>(iters: usize, mut f: impl FnMut() -> T) -> Vec<Duration> {
    f();
    let mut samples = Vec::with_capacity(iters);
    for _ in 0..iters {
        let start = Instant::now();
        std::hint::black_box(f());
        samples.push(start.elapsed());
    }
    samples
}

#[test]
#[ignore = "profiling harness: run with --ignored --nocapture"]
fn selection_move_drag_costs() {
    let sel = rect_selection();
    let coverage = crop_selection(
        &sel.data,
        W,
        H,
        PsdRect {
            top: 0,
            left: 0,
            bottom: H as i32,
            right: W as i32,
        },
    );

    report(
        "selection bounds row scan (hover)",
        &mut time(10, || selection_bounds_extents(&sel)),
    );
    report(
        "Selection::translate (commit only)",
        &mut time(10, || sel.translate(37, 41)),
    );
    report(
        "crop_selection 4000x4000 (release)",
        &mut time(5, || {
            crop_selection(
                &sel.data,
                W,
                H,
                PsdRect {
                    top: 0,
                    left: 0,
                    bottom: H as i32,
                    right: W as i32,
                },
            )
        }),
    );

    report(
        "selection contour retrace (release)",
        &mut time(3, || pictura_select::contour(&sel, 128)),
    );

    let mut doc = masked_document();
    report(
        "full composite (release)",
        &mut time(3, || pictura_render::composite_rgba(&doc)),
    );
    let dirty = PsdRect {
        top: SEL_Y as i32,
        left: SEL_X as i32,
        bottom: (SEL_Y + SEL_H) as i32 + 41,
        right: (SEL_X + SEL_W) as i32 + 37,
    };
    report(
        "region composite 2037² (app default)",
        &mut time(3, || {
            pictura_render::composite_region_active(&doc, dirty, true)
        }),
    );
    println!(
        "region composite backend: {}",
        if pictura_render::gpu_available() {
            "gpu available"
        } else {
            "cpu only (no Vulkan device in this environment)"
        }
    );
    report(
        "move_mask_selection_content (release)",
        &mut time(5, || {
            move_mask_selection_content(&mut doc, "0", &coverage, 37, 41, false, 0)
        }),
    );
}
