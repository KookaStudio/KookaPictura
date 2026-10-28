use super::helpers::rgba_from_argb;
use super::helpers_composite::buffer_to_image;
use super::tests::pixel_layer;
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, ColorMode, Document, PixelBuffer, PsdRect};
use pictura_render::{Planes, ViewPyramid};

/// Viewport the canvas presents into (a typical desktop canvas).
const VIEW: (u32, u32) = (1280, 800);

fn ms(label: &str, d: std::time::Duration) {
    println!("{label}: {:.2} ms", d.as_secs_f64() * 1000.0);
}

/// A one-layer checkerboard (8 px cells) so a Quick Selection flood covers a
/// small region inside a large document.
fn checker_layer(name: &str, w: u32, h: u32) -> pictura_core::Layer {
    let mut layer = pixel_layer(name, w, h, (0, 0, 0));
    let mut r = vec![0u8; (w * h) as usize];
    let mut g = vec![0u8; (w * h) as usize];
    let mut b = vec![0u8; (w * h) as usize];
    for y in 0..h as usize {
        for x in 0..w as usize {
            let v = if (x / 8 + y / 8) % 2 == 0 { 200 } else { 40 };
            let i = y * w as usize + x;
            r[i] = v;
            g[i] = v;
            b[i] = v;
        }
    }
    layer.channels[0].data = r.into();
    layer.channels[1].data = g.into();
    layer.channels[2].data = b.into();
    layer
}

/// Print-only 4000² Quick Selection profile: the press path (composite,
/// flood, combine, contour) and the intra-region move lookup. Run with
/// `cargo test -p pictura_app quick_selection_profile_4000 -- --ignored --nocapture`.
#[test]
#[ignore = "4000x4000 Quick Selection profile; run explicitly with --ignored --nocapture"]
fn quick_selection_profile_4000() {
    let n = 4000u32;
    let mut doc = Document::new(n, n, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![checker_layer("checker", n, n)]);

    let t = std::time::Instant::now();
    let buffer = super::helpers_composite::current_buffer(&doc, false);
    let composite = t.elapsed().as_secs_f64() * 1000.0;

    let t = std::time::Instant::now();
    let first = pictura_select::magic_wand(&buffer, 4, 4, 32, true).expect("flood");
    let flood = t.elapsed().as_secs_f64() * 1000.0;

    let mut add = first.clone();
    let t = std::time::Instant::now();
    add.combine_with(&first, pictura_select::CombineMode::Add);
    let combine = t.elapsed().as_secs_f64() * 1000.0;

    let t = std::time::Instant::now();
    let _ = pictura_select::contour(&add, 128);
    let contour = t.elapsed().as_secs_f64() * 1000.0;

    let t = std::time::Instant::now();
    let second = pictura_select::magic_wand(&buffer, 20, 4, 32, true).expect("flood 2");
    add.combine_with(&second, pictura_select::CombineMode::Add);
    let new_region = t.elapsed().as_secs_f64() * 1000.0;

    let t = std::time::Instant::now();
    let _ = pictura_select::contour(&add, 128);
    let new_contour = t.elapsed().as_secs_f64() * 1000.0;

    let steps = 1000usize;
    let t = std::time::Instant::now();
    let mut hits = 0usize;
    for i in 0..steps {
        let x = 4 + (i % 4) as u32;
        let y = 4 + ((i / 4) % 4) as u32;
        let index = y as usize * n as usize + x as usize;
        if add.data[index] == 255 {
            hits += 1;
        }
    }
    let intra_move = t.elapsed().as_secs_f64() * 1e6 / steps as f64;

    // The per-stage times above plus the totals the change's latency targets
    // use; every non-skipped dab recomposites, so both totals include the
    // composite once.
    let first_dab = composite + flood + combine + contour;
    let new_region_total = composite + new_region + new_contour;

    println!(
        "quick_selection_profile_4000 composite={composite:.2}ms flood={flood:.2}ms \
         combine={combine:.2}ms contour={contour:.2}ms new_region={new_region:.2}ms \
         new_contour={new_contour:.2}ms first_dab={first_dab:.2}ms \
         new_region_total={new_region_total:.2}ms intra_move={intra_move:.2}us hits={hits}"
    );
}

/// Print-only 4000² opacity preview profile: a 512² bounded layer's region
/// composite against the full-document composite the old preview path ran per
/// tick. Run with `cargo test -p pictura_app opacity_preview_profile_4000 --
/// --ignored --nocapture`.
#[test]
#[ignore = "4000x4000 opacity preview profile; run explicitly with --ignored --nocapture"]
fn opacity_preview_profile_4000() {
    let mut doc = Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("base", 4000, 4000, (30, 60, 90)),
        pixel_layer("top", 512, 512, (200, 100, 50)),
    ]);
    let rect = doc.layers.last().unwrap().rect;
    let steps = 3usize;
    let start = std::time::Instant::now();
    for _ in 0..steps {
        let _ = pictura_render::composite_active(&doc, false);
    }
    let full = start.elapsed().as_secs_f64() * 1000.0 / steps as f64;
    let start = std::time::Instant::now();
    for _ in 0..steps {
        let _ = pictura_render::composite_region_active(&doc, rect, false);
    }
    let region = start.elapsed().as_secs_f64() * 1000.0 / steps as f64;
    println!("opacity_preview_profile_4000 full={full:.2}ms region={region:.2}ms steps={steps}");
}

/// The C++ `ImageView::presentLevelForZoom` formula, so the profile crops the
/// level the canvas would. ponytail: mirrored, not shared across the FFI.
fn present_level(zoom: f64, level_count: usize) -> usize {
    let mut level = 0;
    while level + 1 < level_count && 1.0 / (1u64 << (level + 1)) as f64 >= zoom {
        level += 1;
    }
    level
}

/// A naive nearest-neighbour rescale of the full-resolution composite to a
/// viewport-sized buffer: the old present path that read every source row.
fn naive_rescale(src: &Planes<'_>, w: u32, h: u32) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        let sy = (y as u64 * src.height as u64 / h as u64) as usize;
        for x in 0..w {
            let sx = (x as u64 * src.width as u64 / w as u64) as usize;
            let i = sy * src.width as usize + sx;
            let o = ((y * w + x) * 4) as usize;
            out[o] = src.r[i];
            out[o + 1] = src.g[i];
            out[o + 2] = src.b[i];
            out[o + 3] = src.a[i];
        }
    }
    out
}

fn scroll_zoom_pan_profile(n: u32) {
    let plane = n as usize * n as usize;
    let mut data = vec![0u8; plane * 4];
    // Sparse touch so the pages are resident and the premultiply/average is not
    // a no-op, without an O(n²) fill. ponytail: sparse fill, real pixels if the
    // timing ever needs texture.
    for i in (0..plane).step_by(4096) {
        data[i] = (i % 251) as u8;
        data[3 * plane + i] = 255;
    }
    let mut doc = Document::new(n, n, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = PixelBuffer {
        width: n,
        height: n,
        channels: 4,
        data: data.into(),
    };
    let planes = Planes {
        width: n,
        height: n,
        r: &doc.composite.data[..plane],
        g: &doc.composite.data[plane..2 * plane],
        b: &doc.composite.data[2 * plane..3 * plane],
        a: &doc.composite.data[3 * plane..4 * plane],
    };

    let t = std::time::Instant::now();
    let pyramid = ViewPyramid::rebuild(planes);
    ms(&format!("scroll_zoom_pan_profile_{n} rebuild"), t.elapsed());

    let zoom = f64::min(VIEW.0 as f64 / n as f64, VIEW.1 as f64 / n as f64);
    let level = present_level(zoom, pyramid.level_count());
    let (lw, lh) = pyramid.level_size(level);
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: lh as i32,
        right: lw as i32,
    };
    let t = std::time::Instant::now();
    let crop = pyramid.crop(planes, level, rect);
    ms(
        &format!(
            "scroll_zoom_pan_profile_{n} level_crop {}x{}",
            crop.width(),
            crop.height()
        ),
        t.elapsed(),
    );

    let t = std::time::Instant::now();
    let _rescaled = naive_rescale(&planes, VIEW.0, VIEW.1);
    ms(
        &format!("scroll_zoom_pan_profile_{n} full_rescale"),
        t.elapsed(),
    );
}

/// Print-only evidence that a viewport-sized pyramid-level crop is cheaper than
/// rescaling the full-resolution composite. No pass/fail budget (the reference
/// machine is not pinned).
#[test]
#[ignore = "4000x4000 crop-vs-rescale profile; run explicitly with --ignored --nocapture"]
fn scroll_zoom_pan_profile_4000() {
    scroll_zoom_pan_profile(4000);
}

#[test]
#[ignore = "16000x16000 crop-vs-rescale profile; run explicitly with --ignored --nocapture"]
fn scroll_zoom_pan_profile_16000() {
    scroll_zoom_pan_profile(16000);
}

/// Print-only split of the per-stroke cost into stroke start, rasterization and
/// region present, so the brush-latency work is ordered by measurement. The
/// existing `paint_dab_profile_4000` times only the region composite; this adds
/// the two unmeasured components. No pass/fail budget (the reference machine is
/// not pinned).
#[test]
#[ignore = "4000x4000 stroke split profile; run explicitly with --ignored --nocapture"]
fn stroke_split_profile_4000() {
    use pictura_paint::{spacing::SpacingMode, Stroke, StrokeConfig, StrokeSample};

    for layers in [1usize, 8] {
        let mut doc = Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(
            (0..layers)
                .map(|i| pixel_layer(&format!("L{i}"), 4000, 4000, (30, 60, 90)))
                .collect(),
        );
        // Prime the adapter, allocator and pipelines once, untimed.
        let _ = pictura_render::composite_active(&doc, true);
        let gpu = pictura_render::gpu_available();

        // `Document::clone` is a refcount bump since the planes became
        // copy-on-write: this is what a stroke start and a history state pay.
        let t = std::time::Instant::now();
        let cloned = doc.clone();
        let clone = t.elapsed();
        drop(cloned);

        for diameter in [64u32, 500] {
            let cfg = StrokeConfig {
                color: rgba_from_argb(0xFFFF0000),
                diameter,
                hardness: 100,
                spacing: SpacingMode::Fixed(25),
                ..StrokeConfig::default()
            };

            // Each diameter paints its own copy; the clone shares every plane,
            // so the first dab still pays the plane fork a live stroke no
            // longer does (its document shares nothing with the history).
            let mut work = doc.clone();
            // Stroke start: the coverage buffers and the saved-tile bookkeeping.
            let t = std::time::Instant::now();
            let mut stroke = Stroke::begin_at(&work, "0", cfg).expect("stroke begins");
            let begin = t.elapsed();

            // Rasterization: the first sample places exactly one dab, unrefreshed.
            let t = std::time::Instant::now();
            let changed = stroke.sample(
                &mut work,
                StrokeSample {
                    x: 2000.0,
                    y: 2000.0,
                    pressure: 1.0,
                },
            );
            let raster = t.elapsed();
            let rect = stroke.take_dirty().expect("a dab dirties a region");

            // Steady state: the second dab is a non-overlapping one on a stroke
            // whose painted planes have already forked off the document, so only
            // rasterization remains (the first dab also pays that one-time fork).
            let t = std::time::Instant::now();
            let steady_changed = stroke.sample(
                &mut work,
                StrokeSample {
                    x: 2600.0,
                    y: 2000.0,
                    pressure: 1.0,
                },
            );
            let steady = t.elapsed();
            let steady_rect = stroke.take_dirty();

            // Present: the region composite and display conversion `refresh_region` runs.
            let t = std::time::Instant::now();
            let (buffer, _) = pictura_render::composite_region_active(&work, rect, gpu);
            let composite = t.elapsed();
            let t = std::time::Instant::now();
            let _ = buffer_to_image(&buffer);
            let display = t.elapsed();

            println!(
                "stroke_split_profile layers={layers} diameter={diameter} clone={:.3}ms \
                 begin={:.2}ms raster={:.2}ms steady={:.2}ms composite={:.2}ms \
                 display={:.2}ms present={:.2}ms region={}x{} changed={changed} \
                 steady_changed={steady_changed} steady_rect={steady_rect:?}",
                clone.as_secs_f64() * 1000.0,
                begin.as_secs_f64() * 1000.0,
                raster.as_secs_f64() * 1000.0,
                steady.as_secs_f64() * 1000.0,
                composite.as_secs_f64() * 1000.0,
                display.as_secs_f64() * 1000.0,
                (composite + display).as_secs_f64() * 1000.0,
                rect.width(),
                rect.height(),
            );
            drop(stroke);
        }
    }
}

/// Print-only split of the commit (`refresh_regions`) for a dense stroke whose
/// tiles collapse to one bounding-box rectangle, so the dominant sub-step is
/// chosen by measurement. 4000², 1/2/8 full-canvas layers, CPU and GPU region
/// composite, over increasing union sides. The old union-crop image and the new
/// region-buffer image are timed back to back on the same patched state. No
/// pass/fail budget.
#[test]
#[ignore = "4000x4000 commit refresh profile; run explicitly with --ignored --nocapture"]
fn commit_refresh_profile_4000() {
    use super::helpers::{paint_timing, union_rect};
    use super::helpers_composite::premultiplied_display_image;
    use super::PictureViewRust;
    use std::time::{Duration, Instant};

    for layers in [1usize, 2, 8] {
        for side in [1000i32, 2000, 4000] {
            for gpu in [false, true] {
                if gpu && !pictura_render::gpu_available() {
                    continue;
                }
                let mut doc = Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight);
                doc.layers = pictura_core::LayerStack::from_layers(
                    (0..layers)
                        .map(|i| pixel_layer(&format!("L{i}"), 4000, 4000, (30, 60, 90)))
                        .collect(),
                );
                doc.composite = pictura_render::composite_active(&doc, gpu).0;
                let mut rust = PictureViewRust {
                    session: crate::cxxqt_object::document_session::DocumentSession::with_document(
                        doc,
                    ),
                    gpu_compute: gpu,
                    ..Default::default()
                };
                rust.reset_pyramid();

                let union = PsdRect {
                    top: 0,
                    left: 0,
                    bottom: side,
                    right: side,
                };
                rust.session.live.mark_stroke_tile(union);
                let regions = rust.take_stroke_regions(union);

                paint_timing::start("commit refresh profile");
                let t0 = Instant::now();
                let mut covered: Option<PsdRect> = None;
                let mut srgb_image = Duration::ZERO;
                for &rect in &regions {
                    let Some((x0, y0, clipped, buf)) = rust.refresh_region_buffer(rect) else {
                        continue;
                    };
                    let t = Instant::now();
                    let _new = premultiplied_display_image(&buf);
                    if regions.len() == 1 {
                        srgb_image = t.elapsed();
                    }
                    rust.apply_refreshed_region(x0, y0, clipped, buf);
                    covered = Some(covered.map_or(clipped, |u| union_rect(u, clipped)));
                }
                let loop_time = t0.elapsed();
                let covered = covered.expect("a dense stroke covers pixels");
                let t = Instant::now();
                let _old = rust.display_crop(0, covered);
                let crop = t.elapsed();
                paint_timing::record("commit_refresh_region_old", loop_time + crop);
                paint_timing::record("commit_refresh_region_new", loop_time + srgb_image);
                paint_timing::report();
                println!(
                    "commit_refresh layers={layers} side={side} gpu={} regions={} \
                     crop={:.2}ms srgb_image={:.2}ms old={:.2}ms new={:.2}ms delta={:+.2}ms",
                    gpu as u8,
                    regions.len(),
                    crop.as_secs_f64() * 1000.0,
                    srgb_image.as_secs_f64() * 1000.0,
                    (loop_time + crop).as_secs_f64() * 1000.0,
                    (loop_time + srgb_image).as_secs_f64() * 1000.0,
                    (srgb_image.as_secs_f64() - crop.as_secs_f64()) * 1000.0,
                );
            }
        }
    }
}

/// Print-only: how one dab's rasterization and present grow with brush diameter
/// on a 4000² document, so the reduced-resolution stroke preview is sized from
/// a measurement rather than a guess. The one-time plane fork is reported
/// separately because it is a fixed 64 MB cost, independent of the brush.
#[test]
#[ignore = "4000x4000 large-brush profile; run explicitly with --ignored --nocapture"]
fn large_brush_profile_4000() {
    use pictura_paint::{spacing::SpacingMode, Stroke, StrokeConfig, StrokeSample};

    let mut doc = Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight);
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("base", 4000, 4000, (30, 60, 90))]);
    let _ = pictura_render::composite_active(&doc, true);
    let gpu = pictura_render::gpu_available();

    let t = std::time::Instant::now();
    let mut forked = doc.clone();
    // `as_mut_slice` goes through `DerefMut`, so it forks a shared plane the
    // way the stroke's first write does; touching no byte keeps it a pure fork.
    for c in &mut forked.layers[0].channels {
        let _ = c.data.as_mut_slice();
    }
    let fork = t.elapsed();
    drop(forked);

    for diameter in [500u32, 1000, 2000, 5000] {
        let cfg = StrokeConfig {
            color: rgba_from_argb(0xFFFF0000),
            diameter,
            hardness: 100,
            spacing: SpacingMode::Fixed(25),
            ..StrokeConfig::default()
        };
        let mut work = doc.clone();
        let t = std::time::Instant::now();
        let mut stroke = Stroke::begin_at(&work, "0", cfg).expect("stroke begins");
        let begin = t.elapsed();

        let t = std::time::Instant::now();
        let changed = stroke.sample(
            &mut work,
            StrokeSample {
                x: 2000.0,
                y: 2000.0,
                pressure: 1.0,
            },
        );
        let raster = t.elapsed();
        let rect = stroke.take_dirty().expect("a dab dirties a region");

        let t = std::time::Instant::now();
        let (buffer, _) = pictura_render::composite_region_active(&work, rect, gpu);
        let composite = t.elapsed();
        let t = std::time::Instant::now();
        let _ = buffer_to_image(&buffer);
        let display = t.elapsed();

        println!(
            "large_brush_profile fork={:.2}ms diameter={diameter} begin={:.2}ms \
             raster={:.2}ms present={:.2}ms region={}x{} changed={changed}",
            fork.as_secs_f64() * 1000.0,
            begin.as_secs_f64() * 1000.0,
            raster.as_secs_f64() * 1000.0,
            (composite + display).as_secs_f64() * 1000.0,
            rect.width(),
            rect.height(),
        );
        drop(stroke);
    }
}

/// Print-only: every stroke-boundary cost on a document the size of the
/// 16507×16196 world map (issue #185) — the first dab, the in-stroke dabs, the
/// commit refresh, the history capture, and an undo with its canvas rebuild —
/// so each is measured on the engine alone, without the GUI. No pass/fail budget.
#[test]
#[ignore = "16507x16196 stroke-boundary profile; run explicitly with --ignored --nocapture"]
fn large_document_stroke_profile() {
    use super::helpers::paint_timing;
    use super::PictureViewRust;
    use pictura_paint::{spacing::SpacingMode, Stroke, StrokeConfig, StrokeSample};
    use std::time::Instant;

    let (w, h) = (16507u32, 16196u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("Background", w, h, (30, 60, 90))]);
    doc.composite = pictura_render::composite_active(&doc, false).0;
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        ..Default::default()
    };
    rust.reset_pyramid();
    let t = Instant::now();
    let r = &mut rust;
    r.session.history.capture_live(
        r.session.doc.as_mut().expect("document"),
        &None,
        &r.layer_target,
        "Open",
    );
    println!(
        "large_document_open history_capture={:.2}ms",
        t.elapsed().as_secs_f64() * 1000.0
    );

    let cfg = StrokeConfig {
        color: rgba_from_argb(0xFFFF0000),
        diameter: 40,
        hardness: 100,
        spacing: SpacingMode::Fixed(25),
        ..StrokeConfig::default()
    };
    let dab = |rust: &mut PictureViewRust, i: u32| -> f64 {
        let t = Instant::now();
        let stroke = rust.session.live.stroke_mut().expect("stroke");
        stroke.sample(
            rust.session.doc.as_mut().expect("document"),
            StrokeSample {
                x: 8000.0 + i as f32 * 12.0,
                y: 8000.0 + i as f32 * 8.0,
                pressure: 1.0,
            },
        );
        if let Some(rect) = stroke.take_dirty() {
            rust.session.live.mark_stroke_tile(rect);
            if let Some((x0, y0, clipped, srgb)) = rust.refresh_region_buffer(rect) {
                rust.apply_refreshed_region(x0, y0, clipped, srgb);
            }
        }
        t.elapsed().as_secs_f64() * 1000.0
    };

    for round in 0..2 {
        paint_timing::start("large document stroke");
        let t = Instant::now();
        let stroke = Stroke::begin_at(rust.session.doc.as_ref().expect("document"), "0", cfg);
        let session = crate::cxxqt_object::document_session::StrokeSession {
            stroke: Some(stroke.expect("stroke begins")),
            ..Default::default()
        };
        rust.session.live.set_stroke_session(session);
        rust.session.live.reset_stroke_tiles(w, h);
        let begin = t.elapsed().as_secs_f64() * 1000.0;
        let first = dab(&mut rust, 0);
        let rest: Vec<f64> = (1..120).map(|i| dab(&mut rust, i)).collect();
        let worst = rest.iter().copied().fold(0.0, f64::max);
        let avg = rest.iter().sum::<f64>() / rest.len() as f64;

        let t = Instant::now();
        let dirty = rust
            .session
            .live
            .take_stroke_session()
            .expect("stroke session")
            .stroke
            .expect("stroke")
            .finish()
            .expect("painted");
        let regions = rust.take_stroke_regions(dirty);
        let _ = rust.refresh_regions(&regions);
        let refresh = t.elapsed().as_secs_f64() * 1000.0;

        let t = Instant::now();
        let r = &mut rust;
        r.session.history.capture_live(
            r.session.doc.as_mut().expect("document"),
            &None,
            &r.layer_target,
            "Brush",
        );
        let capture = t.elapsed().as_secs_f64() * 1000.0;
        paint_timing::report();

        println!(
            "large_document_stroke round={round} begin={begin:.2}ms first_dab={first:.2}ms \
             dab_avg={avg:.3}ms dab_max={worst:.2}ms commit_refresh={refresh:.2}ms \
             history_capture={capture:.2}ms regions={}",
            regions.len()
        );
    }

    let t = Instant::now();
    let r = &mut rust;
    let restored = r
        .session
        .history
        .undo_live(
            r.session.doc.as_mut().expect("document"),
            &mut r.session.selection,
            &mut r.layer_target,
        )
        .expect("a stroke to undo");
    let history_undo = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    let crate::history::Restored::Region(rect) = restored else {
        panic!("undoing a stroke is bounded, got {restored:?}");
    };
    let _ = rust.refresh_from_composite(rect);
    let refresh = t.elapsed().as_secs_f64() * 1000.0;
    println!(
        "large_document_undo history_undo={history_undo:.2}ms canvas_refresh={refresh:.2}ms \
         region={}x{}",
        rect.width(),
        rect.height()
    );
}

/// Print-only: structural-edit history cost on a 6000×4000 document with
/// twenty full-canvas layers (#973) — one New Layer capture and its undo per
/// round, plus retained tracked-plane bytes after twenty rounds. The retained
/// figure counts tracked-plane allocations only; tile deltas are not included
/// (this profile creates none). No pass/fail timing budget; the deterministic
/// byte bound is asserted in the history unit test.
#[test]
#[ignore = "6000x4000 twenty-layer structural-edit profile; run explicitly with --ignored --nocapture"]
fn large_document_structural_edit_profile() {
    use crate::history::{History, Restored};
    use pictura_select::Selection;
    use std::time::Instant;

    let (w, h) = (6000u32, 4000u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(
        (0..20)
            .map(|i| pixel_layer(&format!("Layer {i}"), w, h, ((i * 11) as u8, 60, 90)))
            .collect(),
    );
    doc.composite = pictura_render::composite_active(&doc, false).0;
    let mut history = History::default();
    let mut selection: Option<Selection> = None;
    let mut target = LayerTarget::default();

    let t = Instant::now();
    history.capture_live(&mut doc, &selection, &target, "Open");
    let base = history.retained_plane_bytes();
    println!(
        "structural_open capture={:.2}ms retained_plane={:.2}GB",
        t.elapsed().as_secs_f64() * 1000.0,
        base as f64 / 1e9
    );

    let (mut worst_add, mut worst_undo) = (0.0f64, 0.0f64);
    for i in 0..20 {
        let t = Instant::now();
        assert!(pictura_render::add_layer(&mut doc, -1, &format!("Layer {}", 20 + i)) >= 0);
        let add = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        history.capture_live(&mut doc, &selection, &target, "New Layer");
        let capture = t.elapsed().as_secs_f64() * 1000.0;
        worst_add = worst_add.max(add + capture);

        let t = Instant::now();
        assert_eq!(
            history.undo_live(&mut doc, &mut selection, &mut target),
            Some(Restored::Everywhere)
        );
        let undo = t.elapsed().as_secs_f64() * 1000.0;
        worst_undo = worst_undo.max(undo);
        assert_eq!(
            history.redo_live(&mut doc, &mut selection, &mut target),
            Some(Restored::Everywhere)
        );
        println!(
            "structural_round round={i} add={add:.2}ms capture={capture:.2}ms undo={undo:.2}ms"
        );
    }

    let retained = history.retained_plane_bytes();
    println!(
        "structural_edit worst_add_capture={worst_add:.2}ms worst_undo={worst_undo:.2}ms \
         retained_plane={:.2}GB base_plane={:.2}GB targets add_capture<20.00ms \
         undo<30.00ms retained<5.00GB",
        retained as f64 / 1e9,
        base as f64 / 1e9,
    );
}

/// Print-only: the Filter dialog's costs on a document the size of the
/// 16507×16196 world map (issue #152) — the dialog's full-image reads, a
/// section preview at fit and at 100% zoom, a re-preview after a parameter
/// change, cancel, and the commit with its refresh and history capture.
/// `PICTURA_PROFILE_GPU=1` runs the filters and composites on the GPU.
#[test]
#[ignore = "16507x16196 filter-dialog profile; run explicitly with --ignored --nocapture"]
fn large_document_filter_profile() {
    use super::filter_map::filter_from_kind_params;
    use super::helpers_composite::premultiplied_display_image;
    use super::impl_filters::{apply_filter_active_region, cancel_filter_preview};
    use super::PictureViewRust;
    use std::time::Instant;

    let (w, h) = (16507u32, 16196u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel_layer("Background", w, h, (30, 60, 90));
    for (c, ch) in layer.channels.iter_mut().filter(|c| c.id >= 0).enumerate() {
        let plane = ch.data.as_mut_slice();
        for (i, px) in plane.iter_mut().enumerate() {
            let (x, y) = (i % w as usize, i / w as usize);
            *px = ((x * 7 + y * 13 + c * 53) % 256) as u8;
        }
    }
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc.composite = pictura_render::composite_active(&doc, false).0;
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("0"),
        gpu_compute: std::env::var_os("PICTURA_PROFILE_GPU").is_some(),
        ..Default::default()
    };
    rust.reset_pyramid();
    let r = &mut rust;
    r.session.history.capture_live(
        r.session.doc.as_mut().expect("document"),
        &None,
        &r.layer_target,
        "Open",
    );

    let timed = |label: &str, f: &mut dyn FnMut()| {
        let t = Instant::now();
        f();
        ms(label, t.elapsed());
    };
    let refresh = |rust: &mut PictureViewRust, rect: PsdRect| {
        if let Some((x0, y0, clipped, srgb)) = rust.refresh_region_buffer(rect) {
            rust.apply_refreshed_region(x0, y0, clipped, srgb);
        }
    };

    let fit_scale = f64::from(VIEW.0) / f64::from(w);
    for (kind, filter) in [
        (
            "gaussian-blur",
            pictura_filters::Filter::GaussianBlur { radius: 0.3 },
        ),
        ("median", pictura_filters::Filter::Median { radius: 1 }),
    ] {
        let whole = PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        };
        for pass in ["build + render", "render"] {
            let t = Instant::now();
            let shown =
                super::filter_tools::canvas_proxy_buffer(&mut rust, &filter, whole, fit_scale)
                    .expect("proxy");
            println!(
                "{kind} canvas proxy at fit, {pass}: {:.0}ms ({}x{})",
                t.elapsed().as_secs_f64() * 1000.0,
                shown.width,
                shown.height
            );
        }
    }
    rust.canvas_proxy = None;

    // A commit on a worker: how long it runs, and what the UI thread still
    // does when it lands.
    for (kind, filter) in [
        (
            "gaussian-blur",
            pictura_filters::Filter::GaussianBlur { radius: 4.0 },
        ),
        (
            "accented-edges",
            pictura_filters::Filter::AccentedEdges {
                edge_width: 9,
                edge_brightness: 38,
                smoothness: 5,
            },
        ),
    ] {
        use super::filter_tools::jobs::{commit_finish, commit_start};
        let t = Instant::now();
        assert!(commit_start(
            &mut rust,
            vec![filter],
            format!("Filter: {kind}"),
            None
        ));
        let started = t.elapsed();
        while rust.filter_jobs.commit_running() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let worker = t.elapsed();
        let t = Instant::now();
        assert!(commit_finish(&mut rust).is_ok());
        println!(
            "{kind} background commit: start (UI)={:.0}ms worker={:.0}ms land (UI)={:.0}ms",
            started.as_secs_f64() * 1000.0,
            worker.as_secs_f64() * 1000.0,
            t.elapsed().as_secs_f64() * 1000.0
        );
    }

    timed("dialog image() (full QImage)", &mut || {
        let level0 = rust.level0.as_ref().expect("level 0");
        drop(premultiplied_display_image(level0));
    });

    let fit = PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    };
    let (cx, cy) = (w as i32 / 2, h as i32 / 2);
    let actual = PsdRect {
        top: cy - VIEW.1 as i32 / 2,
        left: cx - VIEW.0 as i32 / 2,
        bottom: cy + VIEW.1 as i32 / 2,
        right: cx + VIEW.0 as i32 / 2,
    };
    for (kind, params) in [
        ("gaussian-blur", vec![4.0]),
        ("unsharp-mask", vec![150.0, 2.0, 0.0]),
        ("median", vec![2.0]),
    ] {
        assert!(filter_from_kind_params(kind, &params).is_some(), "{kind}");
        for (zoom, section) in [("fit", fit), ("100%", actual)] {
            for pass in ["first", "re-preview"] {
                let t = Instant::now();
                let region =
                    apply_filter_active_region(&mut rust, kind, &params, false, Some(section))
                        .expect("preview applies");
                let filter = t.elapsed();
                let t = Instant::now();
                if let Some(rect) = region {
                    refresh(&mut rust, rect);
                }
                println!(
                    "{kind} preview {zoom} {pass}: filter={:.0}ms refresh={:.0}ms",
                    filter.as_secs_f64() * 1000.0,
                    t.elapsed().as_secs_f64() * 1000.0
                );
            }
            let t = Instant::now();
            let region = cancel_filter_preview(&mut rust).expect("a preview to cancel");
            let cancel = t.elapsed();
            let t = Instant::now();
            if let Some(rect) = region {
                refresh(&mut rust, rect);
            }
            println!(
                "{kind} cancel {zoom}: restore={:.0}ms refresh={:.0}ms",
                cancel.as_secs_f64() * 1000.0,
                t.elapsed().as_secs_f64() * 1000.0
            );
        }
        apply_filter_active_region(&mut rust, kind, &params, false, Some(actual)).expect("preview");
        let t = Instant::now();
        let region = apply_filter_active_region(&mut rust, kind, &params, true, None)
            .expect("commit applies");
        let filter = t.elapsed();
        let t = Instant::now();
        if let Some(rect) = region {
            refresh(&mut rust, rect);
        }
        let refreshed = t.elapsed();
        let t = Instant::now();
        let r = &mut rust;
        r.session.history.capture_live(
            r.session.doc.as_mut().expect("document"),
            &None,
            &r.layer_target,
            "Filter",
        );
        println!(
            "{kind} commit: filter={:.0}ms refresh={:.0}ms history_capture={:.0}ms",
            filter.as_secs_f64() * 1000.0,
            refreshed.as_secs_f64() * 1000.0,
            t.elapsed().as_secs_f64() * 1000.0
        );
    }
}

/// Print-only: a full Radial Blur committed on a worker on a document the size
/// of `earth_xtralarge.jpg` (8000×8000, issue #152): the worker's time, what
/// the UI thread does when it lands, and the bytes history keeps for it.
#[test]
#[ignore = "8000x8000 radial-blur commit profile; run explicitly with --ignored --nocapture"]
fn radial_blur_commit_profile() {
    use super::filter_tools::jobs::{commit_finish, commit_start};
    use super::PictureViewRust;
    use pictura_filters::{Filter, Quality, RadialMethod};
    use std::time::Instant;

    let (w, h) = (8000u32, 8000u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel_layer("Background", w, h, (30, 60, 90));
    for (c, ch) in layer.channels.iter_mut().filter(|c| c.id >= 0).enumerate() {
        for (i, px) in ch.data.as_mut_slice().iter_mut().enumerate() {
            let (x, y) = (i % w as usize, i / w as usize);
            *px = ((x * 7 + y * 13 + c * 53) % 256) as u8;
        }
    }
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc.composite = pictura_render::composite_active(&doc, false).0;
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("0"),
        gpu_compute: std::env::var_os("PICTURA_PROFILE_GPU").is_some(),
        ..Default::default()
    };
    rust.reset_pyramid();
    let r = &mut rust;
    r.session.history.capture_live(
        r.session.doc.as_mut().expect("document"),
        &None,
        &r.layer_target,
        "Open",
    );

    for method in [RadialMethod::Spin, RadialMethod::Zoom] {
        let filter = Filter::RadialBlur {
            method,
            amount: 100.0,
            center: (0.5, 0.5),
            quality: Quality::Best,
        };
        let t = Instant::now();
        assert!(commit_start(
            &mut rust,
            vec![filter],
            "Filter: Radial Blur".into(),
            None
        ));
        while rust.filter_jobs.commit_running() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let worker = t.elapsed();
        let t = Instant::now();
        assert!(commit_finish(&mut rust).is_ok());
        println!(
            "radial {method:?} 100 Best: worker={:.0}ms land (UI)={:.0}ms history tiles={:.0} MB",
            worker.as_secs_f64() * 1000.0,
            t.elapsed().as_secs_f64() * 1000.0,
            rust.session.history.retained_tile_bytes() as f64 / 1e6
        );
    }
}

#[test]
#[ignore = "8000x6000 marquee commit/move profile; run explicitly with --ignored --nocapture"]
fn marquee_selection_profile() {
    use super::helpers::feathered;
    use super::PictureViewRust;
    use pictura_select::Selection;
    use std::time::Instant;

    let (w, h) = (8000u32, 6000u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("Background", w, h, (30, 60, 90))]);
    doc.composite = pictura_render::composite_active(&doc, false).0;
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        ..Default::default()
    };
    let r = &mut rust;
    r.session.history.capture_live(
        r.session.doc.as_mut().expect("document"),
        &None,
        &r.layer_target,
        "Open",
    );

    for (name, elliptical) in [("rect", false), ("ellipse", true)] {
        let t = Instant::now();
        let shape = if elliptical {
            Selection::ellipse(w, h, 1000, 1000, 5000, 3000)
        } else {
            Selection::rect(w, h, 1000, 1000, 5000, 3000)
        };
        r.session.selection = Some(feathered(shape, 0.0));
        let build = t.elapsed();
        let t = Instant::now();
        r.session.history.capture_live(
            r.session.doc.as_mut().expect("document"),
            &r.session.selection,
            &r.layer_target,
            "Rectangular Marquee",
        );
        let record = t.elapsed();
        let t = Instant::now();
        let loops = pictura_select::contour(r.session.selection.as_ref().expect("selection"), 128);
        let contour = t.elapsed();
        let t = Instant::now();
        let moved = r
            .session
            .selection
            .as_ref()
            .expect("selection")
            .translate(37, 21);
        let translate = t.elapsed();
        let t = Instant::now();
        let moved_loops = pictura_select::contour(&moved, 128).len();
        let moved_contour = t.elapsed();
        println!(
            "marquee_selection {name}: build={:.2}ms record={:.2}ms \
             contour={:.2}ms ({} loops) move_translate={:.2}ms move_contour={:.2}ms ({moved_loops} loops)",
            build.as_secs_f64() * 1000.0,
            record.as_secs_f64() * 1000.0,
            contour.as_secs_f64() * 1000.0,
            loops.len(),
            translate.as_secs_f64() * 1000.0,
            moved_contour.as_secs_f64() * 1000.0,
        );
    }
}

#[test]
#[ignore = "8000x6000 feather profile; run explicitly with --ignored --nocapture"]
fn feather_selection_profile() {
    use super::helpers::feathered;
    use pictura_select::Selection;
    use std::time::Instant;

    let (w, h) = (8000u32, 6000u32);
    for feather in [0.0, 4.0, 10.0, 170.0] {
        let shape = Selection::rect(w, h, 1000, 1000, 5000, 3000);
        let t = Instant::now();
        let out = feathered(shape, feather);
        println!(
            "feather_selection radius={feather}: {:.1}ms ({} bytes)",
            t.elapsed().as_secs_f64() * 1000.0,
            out.data.len()
        );
    }
}

/// Print-only 4000² mask-dab profile: the exact mask raster through the old
/// RGBA stand-in versus the new single-channel document, per dab. No pass/fail
/// budget (the reference machine is not pinned).
#[test]
#[ignore = "4000x4000 mask-dab profile; run explicitly with --ignored --nocapture"]
fn mask_dab_profile_4000() {
    use pictura_core::LayerMask;
    use pictura_paint::{spacing::SpacingMode, Stroke, StrokeConfig, StrokeSample};

    let mut doc = Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel_layer("top", 4000, 4000, (200, 100, 50));
    layer.mask = Some(LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 4000,
            right: 4000,
        },
        default_color: 255,
        data: Some(vec![255u8; 4000 * 4000].into()),
        ..Default::default()
    });
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let cfg = StrokeConfig {
        color: rgba_from_argb(0xFF000000),
        diameter: 512,
        hardness: 100,
        spacing: SpacingMode::Fixed(25),
        ..StrokeConfig::default()
    };
    let time = |mut document: Document| {
        let mut stroke = Stroke::begin_at(&document, "0", cfg).expect("stroke begins");
        let mut total = std::time::Duration::ZERO;
        let mut dabs = 0u32;
        for i in 0..16 {
            let sample = StrokeSample {
                x: 600.0 + i as f32 * 180.0,
                y: 2000.0,
                pressure: 1.0,
            };
            let t = std::time::Instant::now();
            if stroke.sample(&mut document, sample) {
                dabs += 1;
            }
            total += t.elapsed();
            let _ = stroke.take_dirty();
        }
        (dabs, total)
    };

    let (rgba_dabs, rgba_time) = time(pictura_render::mask_document(&doc, "0").unwrap().document);
    let (cov_dabs, cov_time) = time(
        pictura_render::mask_document_coverage(&doc, "0")
            .unwrap()
            .document,
    );
    println!(
        "mask_dab_profile dabs={rgba_dabs} rgba_per_dab={:.2}ms coverage_per_dab={:.2}ms",
        rgba_time.as_secs_f64() * 1000.0 / f64::from(rgba_dabs.max(1)),
        cov_time.as_secs_f64() * 1000.0 / f64::from(cov_dabs.max(1)),
    );
}
