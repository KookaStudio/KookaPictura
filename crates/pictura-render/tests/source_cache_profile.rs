//! Ignored profile for the layer-source cache. Run with:
//! `cargo test -p pictura-render --test source_cache_profile -- --ignored --nocapture`

use std::time::Instant;

use pictura_core::{
    BitDepth, BlendMode, Channel, CharacterAttrs, ColorMode, Document, Layer, LayerStack,
    PixelBuffer, PsdRect, SmartObject, SmartObjectKind, TextStyle, TypeTool, VectorFillRule,
    VectorMask, VectorSubpath,
};

fn region_rect((left, top, width, height): (u32, u32, u32, u32)) -> PsdRect {
    PsdRect {
        top: top as i32,
        left: left as i32,
        bottom: (top + height) as i32,
        right: (left + width) as i32,
    }
}

fn source_doc() -> Document {
    let mut d = Document::new(200, 200, ColorMode::Rgb, BitDepth::Eight);
    let n = 200 * 200;
    d.layers = LayerStack::from_layers(vec![Layer {
        name: "source".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 200,
            right: 200,
        },
        channels: vec![
            Channel {
                id: 0,
                data: vec![180u8; n].into(),
            },
            Channel {
                id: 1,
                data: vec![40u8; n].into(),
            },
            Channel {
                id: 2,
                data: vec![10u8; n].into(),
            },
            Channel {
                id: -1,
                data: vec![255u8; n].into(),
            },
        ],
        ..Default::default()
    }]);
    d.composite = pictura_render::composite_rgba(&d);
    d
}

fn smart_source(rect: PsdRect) -> Layer {
    let payload = pictura_codec::write_psd(&source_doc()).expect("source writes");
    let mut so = {
        let mut so = SmartObject::default();
        so.kind = SmartObjectKind::Embedded;
        so
    };
    so.set_payload(payload);
    Layer {
        name: "smart".into(),
        rect,
        smart_object: Some(so),
        ..Default::default()
    }
}

fn smart_filtered_layer() -> Layer {
    let mut layer = smart_source(PsdRect {
        top: 0,
        left: 0,
        bottom: 2000,
        right: 2000,
    });
    let slots = [
        1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 25.0, 0.0, 0.0, 0.0, 0.0, 50.0, 0.0, 0.0, 0.0, 100.0,
    ];
    pictura_codec::attach_lens_correction_filter(&mut layer, &slots).expect("attaches");
    layer
}

fn text_layer() -> Layer {
    let style = TextStyle {
        font: Some("Arial".into()),
        character: CharacterAttrs {
            size: 48.0,
            fill_color: [0.9, 0.1, 0.1, 1.0],
            ..CharacterAttrs::default()
        },
        ..TextStyle::default()
    };
    let mut layer = Layer {
        name: "text".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 200,
            right: 400,
        },
        type_tool: Some(TypeTool {
            transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            text: "Cache".into(),
            bounds: [0, 0, 400, 200],
            text_desc: Vec::new().into(),
            warp_desc: Vec::new().into(),
            fonts: vec!["Arial".into()],
            style: Some(style),
            vertical: false,
            box_bounds: None,
            path: None,
        }),
        ..Default::default()
    };
    pictura_render::set_layer_style_value(&mut layer, "dropShadow.on", 1.0);
    layer.blend = BlendMode::Normal;
    layer
}

fn profile_doc() -> Document {
    let mut d = Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight);
    d.layers = LayerStack::from_layers(vec![smart_filtered_layer(), text_layer()]);
    d
}

#[test]
#[ignore = "profile: cargo test -p pictura-render --test source_cache_profile -- --ignored --nocapture"]
fn warm_region_is_ten_times_faster() {
    let d = profile_doc();
    const N: usize = 5;
    const REGION: (u32, u32, u32, u32) = (100, 100, 512, 120);

    let cold = {
        let start = Instant::now();
        for _ in 0..N {
            pictura_render::purge_source_cache();
            let _ = pictura_render::composite(pictura_render::CompositeRequest::region(
                &d,
                region_rect(REGION),
                false,
            ))
            .0;
        }
        start.elapsed()
    };
    // Warm the cache, then time repeated warm composites.
    let _ = pictura_render::composite(pictura_render::CompositeRequest::region(
        &d,
        region_rect(REGION),
        false,
    ))
    .0;
    let warm = {
        let start = Instant::now();
        for _ in 0..N {
            let _ = pictura_render::composite(pictura_render::CompositeRequest::region(
                &d,
                region_rect(REGION),
                false,
            ))
            .0;
        }
        start.elapsed()
    };
    let region_counters = pictura_render::metrics::counters();

    pictura_render::purge_source_cache();
    let full_cold = {
        let start = Instant::now();
        let _ = pictura_render::composite_rgba(&d);
        start.elapsed()
    };
    let full_warm = {
        let start = Instant::now();
        let _ = pictura_render::composite_rgba(&d);
        start.elapsed()
    };
    let full_counters = pictura_render::metrics::counters();

    assert!(
        warm * 10 <= cold,
        "cold {cold:?} warm {warm:?}: a warm 512x120 region must be 10x faster"
    );
    assert!(
        full_counters.source_hits > region_counters.source_hits,
        "a repeated full composite hits the cache"
    );
    eprintln!(
        "region {N}x: cold {cold:?} ({:.2} ms/op), warm {warm:?} ({:.2} ms/op), ratio {:.1}x",
        cold.as_secs_f64() * 1000.0 / N as f64,
        warm.as_secs_f64() * 1000.0 / N as f64,
        cold.as_secs_f64() / warm.as_secs_f64()
    );
    eprintln!(
        "full 4000x4000: cold {full_cold:?}, warm {full_warm:?}, ratio {:.1}x",
        full_cold.as_secs_f64() / full_warm.as_secs_f64()
    );
    eprintln!(
        "counters after region: hits={} misses={} bytes={} decodes={}",
        region_counters.source_hits,
        region_counters.source_misses,
        region_counters.source_bytes,
        region_counters.effect_decodes
    );
    eprintln!(
        "counters after full:   hits={} misses={} bytes={} decodes={}",
        full_counters.source_hits,
        full_counters.source_misses,
        full_counters.source_bytes,
        full_counters.effect_decodes
    );
}

fn canvas_styled_layer(size: u32) -> Layer {
    let n = (size as usize) * (size as usize);
    let mut layer = Layer {
        name: "canvas-styled".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: size as i32,
            right: size as i32,
        },
        channels: vec![
            Channel {
                id: 0,
                data: vec![200u8; n].into(),
            },
            Channel {
                id: 1,
                data: vec![60u8; n].into(),
            },
            Channel {
                id: 2,
                data: vec![30u8; n].into(),
            },
            Channel {
                id: -1,
                data: vec![255u8; n].into(),
            },
        ],
        ..Default::default()
    };
    pictura_render::set_layer_style_value(&mut layer, "dropShadow.on", 1.0);
    pictura_render::set_layer_style_value(&mut layer, "dropShadow.size", 20.0);
    layer
}

#[test]
#[ignore = "profile: cargo test -p pictura-render --test source_cache_profile -- --ignored --nocapture"]
fn canvas_sized_styled_present_is_faster_than_a_full_composite() {
    const SIZE: u32 = 4000;
    let mut d = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    d.layers = LayerStack::from_layers(vec![canvas_styled_layer(SIZE)]);
    // A dab invalidates the pass exactly as a stroke present does.
    d.layers[0].channels[3].data.as_mut_slice()[100 * SIZE as usize + 100] = 0;
    let start = Instant::now();
    let region = pictura_render::composite(pictura_render::CompositeRequest::region(
        &d,
        region_rect((100, 100, 128, 128)),
        false,
    ))
    .0;
    let region_time = start.elapsed();
    let start = Instant::now();
    let full = pictura_render::composite_rgba(&d);
    let full_time = start.elapsed();
    let start = Instant::now();
    let _ = pictura_render::composite_rgba(&d);
    let warm_full_time = start.elapsed();
    let fp = (SIZE as usize) * (SIZE as usize);
    for y in 0..128usize {
        for x in 0..128usize {
            let fi = (100 + y) * SIZE as usize + (100 + x);
            let ri = y * 128 + x;
            for c in 0..4 {
                assert_eq!(
                    region.data.as_slice()[c * 128 * 128 + ri],
                    full.data.as_slice()[c * fp + fi],
                    "pixel ({x},{y}) channel {c}"
                );
            }
        }
    }
    eprintln!(
        "canvas 4000x4000 styled (after a dab): cold_full={:.1}ms warm_full={:.1}ms region={:.2}ms ratio={:.1}x",
        full_time.as_secs_f64() * 1000.0,
        warm_full_time.as_secs_f64() * 1000.0,
        region_time.as_secs_f64() * 1000.0,
        full_time.as_secs_f64() / region_time.as_secs_f64()
    );
    assert!(
        region_time * 10 <= full_time,
        "region {region_time:?} full {full_time:?}: a dab-sized present must be 10x faster"
    );
}

const PROFILE_W: u32 = 4934;
const PROFILE_H: u32 = 1500;

fn opaque_pixel_layer(name: &str, size: (u32, u32), rgb: (u8, u8, u8)) -> Layer {
    let n = size.0 as usize * size.1 as usize;
    Layer {
        name: name.into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: size.1 as i32,
            right: size.0 as i32,
        },
        channels: vec![
            Channel {
                id: 0,
                data: vec![rgb.0; n].into(),
            },
            Channel {
                id: 1,
                data: vec![rgb.1; n].into(),
            },
            Channel {
                id: 2,
                data: vec![rgb.2; n].into(),
            },
            Channel {
                id: -1,
                data: vec![255u8; n].into(),
            },
        ],
        blend: BlendMode::Normal,
        ..Default::default()
    }
}

fn polygon_mask(points: &[(i32, i32)]) -> VectorMask {
    VectorMask {
        subpaths: vec![VectorSubpath {
            closed: true,
            operation: 1,
            fill_rule: VectorFillRule::EvenOdd,
            points: points.iter().map(|&(x, y)| [x * 256, y * 256]).collect(),
        }],
        ..Default::default()
    }
}

fn beveled_masked_layer(name: &str, rgb: (u8, u8, u8), points: &[(i32, i32)]) -> Layer {
    let mut layer = opaque_pixel_layer(name, (PROFILE_W, PROFILE_H), rgb);
    layer.vector_mask = Some(polygon_mask(points));
    let _ = pictura_render::set_layer_style_value(&mut layer, "bevel.on", 1.0);
    let _ = pictura_render::set_layer_style_value(&mut layer, "bevel.size", 250.0);
    let _ = pictura_render::set_layer_style_value(&mut layer, "bevel.depth", 1000.0);
    let _ = pictura_render::set_layer_style_value(&mut layer, "bevel.soften", 0.0);
    let _ = pictura_render::set_layer_style_value(&mut layer, "innerGlow.on", 1.0);
    layer
}

fn bevel_fixture() -> Document {
    let mut d = Document::new(PROFILE_W, PROFILE_H, ColorMode::Rgb, BitDepth::Eight);
    d.layers = LayerStack::from_layers(vec![
        opaque_pixel_layer("Background", (PROFILE_W, PROFILE_H), (255, 255, 255)),
        beveled_masked_layer(
            "Red",
            (200, 20, 20),
            &[(300, 250), (2300, 250), (1300, 1250)],
        ),
        beveled_masked_layer(
            "Blue",
            (20, 20, 200),
            &[(2600, 250), (4600, 750), (2600, 1250), (3100, 750)],
        ),
    ]);
    d
}

fn timed_region(
    pool: &rayon::ThreadPool,
    d: &Document,
    region: PsdRect,
) -> (std::time::Duration, PixelBuffer) {
    let start = Instant::now();
    let out = pool.install(|| {
        pictura_render::composite(pictura_render::CompositeRequest::region(d, region, false)).0
    });
    (start.elapsed(), out)
}

struct Round {
    pair: std::time::Duration,
    union: std::time::Duration,
    top: PixelBuffer,
    bottom: PixelBuffer,
    union_out: PixelBuffer,
}

fn round(
    pool: &rayon::ThreadPool,
    d: &Document,
    top: PsdRect,
    bottom: PsdRect,
    union: PsdRect,
    pair_first: bool,
) -> Round {
    if pair_first {
        let (t_top, top_out) = timed_region(pool, d, top);
        let (t_bottom, bottom_out) = timed_region(pool, d, bottom);
        let (t_union, union_out) = timed_region(pool, d, union);
        Round {
            pair: t_top + t_bottom,
            union: t_union,
            top: top_out,
            bottom: bottom_out,
            union_out,
        }
    } else {
        let (t_union, union_out) = timed_region(pool, d, union);
        let (t_top, top_out) = timed_region(pool, d, top);
        let (t_bottom, bottom_out) = timed_region(pool, d, bottom);
        Round {
            pair: t_top + t_bottom,
            union: t_union,
            top: top_out,
            bottom: bottom_out,
            union_out,
        }
    }
}

fn median(mut samples: Vec<std::time::Duration>) -> std::time::Duration {
    samples.sort();
    samples[samples.len() / 2]
}

fn assert_slice(part: &PixelBuffer, full: &PixelBuffer, x0: u32, y0: u32) {
    let pw = part.width as usize;
    let fw = full.width as usize;
    let pplane = pw * part.height as usize;
    let fplane = fw * full.height as usize;
    let pdata = part.data.as_slice();
    let fdata = full.data.as_slice();
    for y in 0..part.height as usize {
        for x in 0..part.width as usize {
            let pi = y * pw + x;
            let fi = (y0 as usize + y) * fw + x0 as usize + x;
            for channel in 0..4 {
                assert_eq!(
                    pdata[channel * pplane + pi],
                    fdata[channel * fplane + fi],
                    "pixel ({x},{y}) channel {channel}"
                );
            }
        }
    }
}

#[test]
#[ignore = "profile: cargo test -p pictura-render --test source_cache_profile warm_bevel_region_replay_profile -- --ignored --exact --nocapture --test-threads=1"]
fn warm_bevel_region_replay_profile() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("one-worker pool");
    let d = bevel_fixture();
    let top = region_rect((0, 512, PROFILE_W, 128));
    let bottom = region_rect((0, 640, PROFILE_W, 128));
    let union = region_rect((0, 512, PROFILE_W, 256));

    pictura_render::purge_source_cache();
    let cold_start = Instant::now();
    let _ = timed_region(&pool, &d, top);
    let cold_short = cold_start.elapsed();

    let full_start = Instant::now();
    let full = pool.install(|| pictura_render::composite_rgba(&d));
    let warm_full = full_start.elapsed();

    let before = pictura_render::metrics::counters();
    let warm_sample = pool.install(|| {
        pictura_render::composite(pictura_render::CompositeRequest::region(&d, top, false)).0
    });
    let after = pictura_render::metrics::counters();
    assert_eq!(
        after.source_hits,
        before.source_hits + 4,
        "a warm short call reuses two effect decodes and two ready passes"
    );
    assert_eq!(
        after.source_misses, before.source_misses,
        "a warm short call has no source misses"
    );
    assert_eq!(
        after.effect_decodes, before.effect_decodes,
        "a warm short call decodes no effect"
    );
    assert!(
        after.source_bytes > 0,
        "the whole-layer passes stay retained"
    );
    assert_slice(&warm_sample, &full, 0, 512);

    for round_index in 0..3 {
        let _ = round(&pool, &d, top, bottom, union, round_index % 2 == 0);
    }

    let mut pair_samples = Vec::new();
    let mut union_samples = Vec::new();
    for round_index in 0..9 {
        let measured = round(&pool, &d, top, bottom, union, round_index % 2 == 0);
        assert_slice(&measured.top, &full, 0, 512);
        assert_slice(&measured.bottom, &full, 0, 640);
        assert_slice(&measured.union_out, &full, 0, 512);
        pair_samples.push(measured.pair);
        union_samples.push(measured.union);
    }

    let pair = median(pair_samples);
    let banded = median(union_samples);
    let ratio = pair.as_secs_f64() / banded.as_secs_f64();
    eprintln!(
        "bevel {PROFILE_W}x{PROFILE_H}: cold short {cold_short:?}, warm full {warm_full:?}, \
         median pair {pair:?}, median union {banded:?}, ratio {ratio:.2}"
    );
    eprintln!(
        "warm short reuse signals: hits +{}, misses +{}, decodes +{}",
        after.source_hits - before.source_hits,
        after.source_misses - before.source_misses,
        after.effect_decodes - before.effect_decodes
    );
    assert!(
        ratio <= 4.0,
        "median pair {pair:?} / union {banded:?} = {ratio:.2}, expected at most 4.0"
    );
}
