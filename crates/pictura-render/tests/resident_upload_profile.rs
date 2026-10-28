//! Ignored profile for the resident upload store. Run with:
//! `cargo test -p pictura-render --test resident_upload_profile -- --ignored --nocapture`
//!
//! Models a live stroke on a 4000² document: one layer is written every present
//! (clearing its stamp), the others keep theirs. Measures the per-present
//! upload reuse, the store size after 300 presents, and the full composite that
//! follows the stroke.

use std::time::Instant;

use pictura_core::{
    fresh_stamp, BitDepth, Channel, ColorMode, Document, Layer, LayerStack, PsdRect,
};
use pictura_render::metrics;

const DOC: u32 = 4000;
const LAYERS: usize = 8;
const PRESENTS: u32 = 300;
const RECT: i32 = 512;

fn whole_rect() -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: DOC as i32,
        right: DOC as i32,
    }
}

fn layer(k: u8) -> Layer {
    let n = (DOC * DOC) as usize;
    Layer {
        name: format!("layer {k}"),
        rect: whole_rect(),
        opacity: 255,
        fill: 255,
        visible: true,
        channels: [
            Channel {
                id: 0,
                data: vec![40 + k * 20; n].into(),
            },
            Channel {
                id: 1,
                data: vec![80 + k * 10; n].into(),
            },
            Channel {
                id: 2,
                data: vec![120 - k * 5; n].into(),
            },
            Channel {
                id: -1,
                data: vec![255; n].into(),
            },
        ]
        .to_vec(),
        ..Default::default()
    }
}

fn profile_doc() -> Document {
    let mut doc = Document::new(DOC, DOC, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers((0..LAYERS as u8).map(layer).collect());
    doc
}

fn moving_rect(frame: u32) -> PsdRect {
    let span = DOC as i32 - RECT;
    let x = (frame as i32 * 37) % span;
    let y = (frame as i32 * 53) % span;
    PsdRect {
        left: x,
        top: y,
        right: x + RECT,
        bottom: y + RECT,
    }
}

fn stamp_layer(layer: &mut Layer) {
    for channel in &mut layer.channels {
        channel.data.set_stamp(fresh_stamp());
    }
}

#[test]
#[ignore = "profile: cargo test -p pictura-render --test resident_upload_profile -- --ignored --nocapture"]
fn resident_uploads_profile() {
    if !pictura_render::gpu_available() {
        eprintln!("no usable Vulkan GPU; skipping resident profile");
        return;
    }
    let mut doc = profile_doc();
    for layer in &mut doc.layers {
        stamp_layer(layer);
    }

    let t = Instant::now();
    let _ = pictura_render::composite_active(&doc, true);
    let seed = t.elapsed();
    let seeded = metrics::counters();

    let before = metrics::counters();
    let t = Instant::now();
    for frame in 0..PRESENTS {
        // The stroke writes the active layer: its stamp clears, so it cannot be
        // reused. The other layers keep their stamps.
        doc.layers[0].channels[0].data.as_mut_slice()[0] ^= 1;
        let _ = pictura_render::composite_region_active(&doc, moving_rect(frame), true);
    }
    let presents = t.elapsed();
    let after = metrics::counters();

    // The stroke commits: the history stamps the written planes afresh.
    stamp_layer(&mut doc.layers[0]);

    let t = Instant::now();
    let _ = pictura_render::composite_active(&doc, true);
    let full = t.elapsed();
    let end = metrics::counters();

    eprintln!(
        "resident profile: {DOC}x{DOC}, {LAYERS} layers, {RECT}x{RECT} rect, {PRESENTS} presents"
    );
    eprintln!(
        "seed full: {seed:.2?}, hits={} misses={}",
        seeded.resident_hits, seeded.resident_misses
    );
    eprintln!(
        "presents: {presents:.2?} ({:.2} ms/present), hits=+{} misses=+{}",
        presents.as_secs_f64() * 1000.0 / f64::from(PRESENTS),
        after.resident_hits - before.resident_hits,
        after.resident_misses - before.resident_misses
    );
    eprintln!(
        "store after presents: entries {} -> {}, bytes {} -> {}",
        before.resident_entries,
        after.resident_entries,
        before.resident_bytes,
        after.resident_bytes
    );
    eprintln!(
        "full after stroke: {full:.2?}, hits=+{} misses=+{}",
        end.resident_hits - after.resident_hits,
        end.resident_misses - after.resident_misses
    );
    eprintln!(
        "store after full: entries={}, bytes={}",
        end.resident_entries, end.resident_bytes
    );
}
