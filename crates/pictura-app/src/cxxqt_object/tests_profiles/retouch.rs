use super::*;
use pictura_paint::pattern;
use pictura_paint::smudge::SmudgeOptions;
use pictura_paint::spacing::SpacingMode;
use pictura_paint::stamp::{
    layer_surface, sample_scope, surface_from_composite, surface_from_composite_rect,
    CloneSampling, StampSource,
};
use pictura_paint::tone::ToneOptions;
use pictura_paint::{Stroke, StrokeConfig, StrokeSample};
use std::time::Instant;

const W: u32 = 8000;
const H: u32 = 6000;

fn document() -> Document {
    let mut doc = Document::new(W, H, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("Background", W, H, (30, 60, 90)),
        pixel_layer("Layer 1", W, H, (90, 60, 30)),
    ]);
    doc.composite =
        pictura_render::composite(pictura_render::CompositeRequest::whole(&doc, false)).0;
    doc
}

fn cfg() -> StrokeConfig {
    StrokeConfig {
        color: rgba_from_argb(0xFFFF0000),
        diameter: 40,
        hardness: 100,
        spacing: SpacingMode::Fixed(25),
        ..StrokeConfig::default()
    }
}

macro_rules! measure {
    ($label:expr, $begin:expr) => {{
        let t = Instant::now();
        let stroke = $begin;
        println!(
            "retouch_stroke_start tool={} begin={:.2}ms",
            $label,
            t.elapsed().as_secs_f64() * 1000.0
        );
        drop(stroke);
    }};
}

/// Print-only: the stroke-start cost of the retouching and source tools on an
/// 8000×6000 document, measured on the engine without the GUI. No budget.
#[test]
#[ignore = "8000x6000 retouch stroke-start profile; run explicitly with --ignored --nocapture"]
fn retouch_stroke_start_profile() {
    let doc = document();
    let cfg = cfg();
    measure!("brush", Stroke::begin_at(&doc, "0", cfg).expect("brush"));
    measure!(
        "dodge",
        Stroke::begin_tone(&doc, "0", cfg, ToneOptions::default()).expect("dodge")
    );
    measure!("smudge_sample_all", {
        let rect = doc.layers[0].rect;
        let sampled = surface_from_composite_rect(&doc.composite, rect);
        Stroke::begin_smudge(&doc, "0", cfg, SmudgeOptions::default(), Some(sampled))
            .expect("smudge")
    });
    measure!("clone_all_layers", {
        let scope = sample_scope(&doc, "0", CloneSampling::AllLayers, false).expect("scope");
        let surface = surface_from_composite(&pictura_render::composite_rgba(&scope));
        Stroke::begin_source(&doc, "0", cfg, StampSource::new(surface, (-100, 0))).expect("clone")
    });
    measure!("history_brush", {
        let surface = layer_surface(&doc, "0").expect("surface");
        Stroke::begin_source(&doc, "0", cfg, StampSource::new(surface, (0, 0))).expect("history")
    });
    measure!("pattern_stamp", {
        let tile = pattern::tile(0).expect("tile");
        let source = StampSource::pattern(tile, (0, 0), W, H).expect("pattern");
        Stroke::begin_source(&doc, "0", cfg, source).expect("pattern")
    });
}

/// One stroke's begin, first dab, and mean later dab, on a document the history
/// shares, as a live one's is.
fn measure_stroke(label: &str, doc: &Document, begin: impl Fn(&Document) -> Stroke) {
    let mut live = doc.clone();
    let t = Instant::now();
    let mut stroke = begin(&live);
    let begun = t.elapsed();
    let at = |i: usize| StrokeSample {
        x: 2000.0 + i as f32 * 150.0,
        y: 8000.0,
        pressure: 1.0,
    };
    let t = Instant::now();
    stroke.sample(&mut live, at(0));
    let first = t.elapsed();
    let t = Instant::now();
    for i in 1..=20 {
        stroke.sample(&mut live, at(i));
    }
    let later = t.elapsed() / 20;
    println!(
        "tone_stroke tool={label} begin={:.2}ms first_dab={:.2}ms later={:.2}ms",
        begun.as_secs_f64() * 1000.0,
        first.as_secs_f64() * 1000.0,
        later.as_secs_f64() * 1000.0
    );
}

/// Print-only: toning against Brush at two sizes on the world map's dimensions,
/// through the engine, without the GUI. No budget.
#[test]
#[ignore = "16507x16196 tone stroke profile; run explicitly with --ignored --nocapture"]
fn tone_stroke_profile() {
    let (w, h) = (16507, 16196);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("Background", w, h, (120, 90, 60))]);
    for diameter in [500, 1940] {
        let cfg = StrokeConfig {
            diameter,
            hardness: 0,
            ..cfg()
        };
        measure_stroke(&format!("brush-{diameter}"), &doc, |d| {
            Stroke::begin_at(d, "0", cfg).expect("brush")
        });
        measure_stroke(&format!("dodge-{diameter}"), &doc, |d| {
            Stroke::begin_tone(d, "0", cfg, ToneOptions::default()).expect("dodge")
        });
    }
}
