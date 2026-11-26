//! Long brush strokes on a 21000² document, one stroke's phases timed apart
//! (#1609): the layer grows, the dabs, and the history record.

use super::super::edit::Edit;
use super::restore::new_layer_view;
use pictura_paint::stroke::Stroke;
use pictura_paint::{Rgba, StrokeConfig, StrokeSample};
use std::time::{Duration, Instant};

/// The #1608 strokes (400 px hard round, about 15,500 px diagonally in 121
/// samples, each its own history state), each split into its phases. `SIDE`
/// and `STROKES` override the document side and the stroke count; the strokes
/// scale with the side.
#[test]
#[ignore = "21000x21000 long-stroke profile; run explicitly with --release --ignored --nocapture"]
fn long_stroke_phases_profile() {
    let env = |name: &str, default: u32| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let side = env("SIDE", 21_000);
    let strokes = env("STROKES", 15) as usize;
    let scale = side as f32 / 21_000.0;
    let (mut view, created, path) = new_layer_view(side);
    let cfg = StrokeConfig {
        color: Rgba {
            r: 200,
            g: 30,
            b: 30,
            a: 255,
        },
        diameter: 400,
        hardness: 100,
        opacity: 100,
        flow: 100,
        ..StrokeConfig::default()
    }
    .sanitized();
    let all = Instant::now();
    for s in 0..strokes {
        let doc = view.session.doc.as_mut().expect("document");
        let mut stroke = Stroke::begin_at(doc, &path, cfg).expect("a stroke");
        let (x0, x1) = (
            (1_500.0 + s as f32 * 800.0) * scale,
            (4_500.0 + s as f32 * 800.0) * scale,
        );
        let (mut grow, mut dabs, mut grows) = (Duration::ZERO, Duration::ZERO, 0);
        for i in 0..=120 {
            let t = i as f32 / 120.0;
            let (x, y) = (x0 + (x1 - x0) * t, (2_000.0 + 15_500.0 * t) * scale);
            let before = doc.layers[created].rect;
            let t = Instant::now();
            assert!(stroke.ensure_rect_for_sample(doc, x, y), "the grow fits");
            grow += t.elapsed();
            grows += usize::from(doc.layers[created].rect != before);
            let t = Instant::now();
            stroke.sample(
                doc,
                StrokeSample {
                    x,
                    y,
                    pressure: 1.0,
                },
            );
            dabs += t.elapsed();
        }
        let rect = stroke.finish().expect("the stroke paints");
        let t = Instant::now();
        view.apply_edit("Brush", Edit::regions(vec![rect], false));
        let record = t.elapsed();
        let layer = &view.session.doc.as_ref().expect("document").layers[created];
        println!(
            "stroke {s}: grow {:.0} ms ({grows} grows), dabs {:.0} ms, record {:.0} ms; layer {}x{}",
            grow.as_secs_f64() * 1e3,
            dabs.as_secs_f64() * 1e3,
            record.as_secs_f64() * 1e3,
            layer.rect.width(),
            layer.rect.height()
        );
    }
    println!(
        "{strokes} strokes painted and recorded: {:.0} ms",
        all.elapsed().as_secs_f64() * 1e3
    );
}
