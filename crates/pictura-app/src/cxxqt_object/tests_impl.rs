use super::helpers::*;
use super::helpers_composite::*;
use super::tests::pixel_layer;
use crate::history::{History, Snapshot};
use pictura_core::{BitDepth, Channel, ColorMode, Document, PsdRect};
use pictura_select::Selection;

/// Print-only evidence that a region refresh no longer pays a per-pixel FFI
/// blit. Replays `refresh_region`'s body for a 512² and a 1024² region on a
/// 4000² document; the old per-pixel `blit_image_region` cost 7.36 ms and
/// 27.3 ms respectively (release, RTX 3090, per the M35 brief).
#[test]
#[ignore = "4000x4000 region profile; run explicitly with --ignored --nocapture"]
fn region_refresh_profile_4000() {
    let mut doc = Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("base", 4000, 4000, (30, 60, 90)),
        pixel_layer("top", 4000, 4000, (200, 100, 50)),
    ]);
    let (_, backend) = pictura_render::composite_active(&doc, true);
    for side in [512u32, 1024u32] {
        let rect = PsdRect {
            top: 1024,
            left: 1024,
            bottom: (1024 + side) as i32,
            right: (1024 + side) as i32,
        };
        let t = std::time::Instant::now();
        let (buffer, _) = pictura_render::composite_region_active(&doc, rect, true);
        let composite = t.elapsed();
        let t = std::time::Instant::now();
        patch_composite_region(&mut doc, &buffer, 1024, 1024);
        let patch = t.elapsed();
        let t = std::time::Instant::now();
        let _ = buffer_to_image(&buffer);
        let convert = t.elapsed();
        println!(
            "region_refresh_profile side={side} composite={:.2}ms patch={:.2}ms \
             convert={:.2}ms total={:.2}ms (old per-pixel blit: 512->7.36ms, 1024->27.3ms)",
            composite.as_secs_f64() * 1000.0,
            patch.as_secs_f64() * 1000.0,
            convert.as_secs_f64() * 1000.0,
            (composite + patch + convert).as_secs_f64() * 1000.0,
        );
    }
    println!("region_refresh_profile backend: {backend:?}");
}

/// Print-only 4000² live-dab profile comparing the CPU region oracle against the
/// GPU region composite for a 512 px brush. It showed the GPU path is ~8× faster
/// per dab on the reference machine, which is why the in-stroke path keeps the
/// preferred backend. No pass/fail budget (the reference machine is not pinned).
#[test]
#[ignore = "4000x4000 live-dab profile; run explicitly with --ignored --nocapture"]
fn paint_dab_profile_4000() {
    use pictura_paint::{spacing::SpacingMode, Stroke, StrokeConfig, StrokeSample};

    let mut doc = Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("base", 4000, 4000, (30, 60, 90)),
        pixel_layer("top", 4000, 4000, (200, 100, 50)),
    ]);
    // Prime the GPU adapter and allocator once, untimed, so the GPU comparison
    // below measures the per-call round-trip rather than first-call setup.
    let (_, prime_backend) = pictura_render::composite_active(&doc, true);
    let gpu_available = pictura_render::gpu_available();

    let cfg = StrokeConfig {
        color: rgba_from_argb(0xFFFF0000),
        diameter: 512,
        hardness: 100,
        spacing: SpacingMode::Fixed(25),
        ..StrokeConfig::default()
    };
    let mut stroke = Stroke::begin_at(&doc, "1", cfg).expect("stroke begins");

    let mut dabs = 0u32;
    let mut cpu_total = std::time::Duration::ZERO;
    let mut gpu_total = std::time::Duration::ZERO;
    for i in 0..16 {
        let sample = StrokeSample {
            x: 600.0 + i as f32 * 180.0,
            y: 2000.0,
            pressure: 1.0,
        };
        if !stroke.sample(&mut doc, sample) {
            continue;
        }
        let Some(rect) = stroke.take_dirty() else {
            continue;
        };
        let t = std::time::Instant::now();
        let (cpu_buffer, cpu_backend) = pictura_render::composite_region_active(&doc, rect, false);
        let cpu_composite = t.elapsed();
        let t = std::time::Instant::now();
        let _ = buffer_to_image(&cpu_buffer);
        let blit = t.elapsed();
        let gpu_composite = if gpu_available {
            let t = std::time::Instant::now();
            let _ = pictura_render::composite_region_active(&doc, rect, true);
            Some(t.elapsed())
        } else {
            None
        };
        dabs += 1;
        cpu_total += cpu_composite + blit;
        gpu_total += gpu_composite.unwrap_or_default();
        println!(
            "paint_dab_profile dab={dabs} region={}x{} cpu_composite={:.2}ms blit={:.2}ms \
             gpu_composite={} backend={cpu_backend:?}",
            cpu_buffer.width,
            cpu_buffer.height,
            cpu_composite.as_secs_f64() * 1000.0,
            blit.as_secs_f64() * 1000.0,
            gpu_composite.map_or_else(
                || "n/a".to_string(),
                |d| format!("{:.2}ms", d.as_secs_f64() * 1000.0)
            ),
        );
    }
    println!(
        "paint_dab_profile dabs={dabs} cpu_per_dab={:.2}ms cpu_total={:.2}ms \
         gpu_per_dab={:.2}ms gpu_total={:.2}ms prime={prime_backend:?}",
        cpu_total.as_secs_f64() * 1000.0 / f64::from(dabs.max(1)),
        cpu_total.as_secs_f64() * 1000.0,
        gpu_total.as_secs_f64() * 1000.0 / f64::from(dabs.max(1)),
        gpu_total.as_secs_f64() * 1000.0,
    );
}

/// Print-only reference for a visibility toggle on a 4000² document whose
/// layer covers only a small rect: the region path composites and patches that
/// rect, versus a full `current_buffer` recomposite of the whole document. No
/// pass/fail budget (the reference machine is not pinned).
#[test]
#[ignore = "4000x4000 visibility profile; run explicitly with --ignored --nocapture"]
fn visibility_region_profile_4000() {
    let ms = |label: &str, d: std::time::Duration| {
        println!(
            "visibility_profile {label} 4000x4000: {:.2} ms",
            d.as_secs_f64() * 1000.0
        );
    };

    let mut small = pixel_layer("small", 200, 200, (200, 100, 50));
    small.rect = PsdRect {
        top: 1900,
        left: 1900,
        bottom: 2100,
        right: 2100,
    };
    let mut doc = Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("base", 4000, 4000, (30, 60, 90)),
        small,
    ]);
    let rendered = current_buffer(&doc, false);
    store_composite(&mut doc, &rendered);

    // Region path: hide the small layer and composite only its rect.
    let (_, region) = set_visible_paths_union(&mut doc, &["1"], false);
    let rect = region.expect("small raster layer is bounded");
    let (x0, y0, ..) = clamp_region(rect, doc.width, doc.height).expect("in bounds");
    let t = std::time::Instant::now();
    let (buffer, backend) = pictura_render::composite_region_active(&doc, rect, false);
    patch_composite_region(&mut doc, &buffer, x0, y0);
    let _ = buffer_to_image(&buffer);
    ms("region toggle", t.elapsed());

    // Full recomposite of the same state, for comparison.
    let mut full = doc.clone();
    let t = std::time::Instant::now();
    let rendered = current_buffer(&full, false);
    store_composite(&mut full, &rendered);
    let _ = buffer_to_image(&full.composite);
    ms("full recomposite", t.elapsed());

    println!("visibility_profile backend: {backend:?}");
}

#[test]
fn filter_from_kind_maps_known_and_rejects_unknown() {
    use pictura_filters::{
        BrushType, ContourEdge, ExtrudeType, Filter, LensType, MezzotintType, NoiseDistribution,
        PolarKind, RippleSize, SharpenRemove, ShearFill, SpherizeMode, TextureOptions, TileFill,
        TonalFade, WaveType, WindMethod, ZigZagStyle,
    };

    assert_eq!(
        filter_from_kind("gaussian-blur"),
        Some(Filter::GaussianBlur { radius: 5.0 })
    );
    assert_eq!(
        filter_from_kind("box-blur"),
        Some(Filter::BoxBlur { radius: 3 })
    );
    assert_eq!(
        filter_from_kind("surface-blur"),
        Some(Filter::SurfaceBlur {
            radius: 10,
            threshold: 20,
        })
    );
    assert_eq!(
        filter_from_kind("motion-blur"),
        Some(Filter::MotionBlur {
            angle: 0.0,
            distance: 15,
        })
    );
    assert_eq!(
        filter_from_kind("median"),
        Some(Filter::Median { radius: 2 })
    );
    assert_eq!(filter_from_kind("despeckle"), Some(Filter::Despeckle));
    assert_eq!(filter_from_kind("sharpen"), Some(Filter::Sharpen));
    assert_eq!(filter_from_kind("sharpen-more"), Some(Filter::SharpenMore));
    assert_eq!(
        filter_from_kind("unsharp-mask"),
        Some(Filter::UnsharpMask {
            amount: 150.0,
            radius: 1.0,
            threshold: 0,
        })
    );
    assert_eq!(
        filter_from_kind("add-noise"),
        Some(Filter::AddNoise {
            amount: 25.0,
            distribution: NoiseDistribution::Uniform,
            monochromatic: false,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("maximum"),
        Some(Filter::Maximum { radius: 2 })
    );
    assert_eq!(
        filter_from_kind("minimum"),
        Some(Filter::Minimum { radius: 2 })
    );
    assert_eq!(
        filter_from_kind("offset"),
        Some(Filter::Offset {
            horizontal: 4,
            vertical: 4,
            wrap: true,
            background: [0, 0, 0],
            transparent: false,
        })
    );
    assert_eq!(
        filter_from_kind("high-pass"),
        Some(Filter::HighPass { radius: 4.0 })
    );
    assert_eq!(
        filter_from_kind("emboss"),
        Some(Filter::Emboss {
            angle: 135.0,
            height: 2.0,
            amount: 100.0,
        })
    );
    assert_eq!(filter_from_kind("find-edges"), Some(Filter::FindEdges));
    assert_eq!(filter_from_kind("solarize"), Some(Filter::Solarize));
    assert_eq!(
        filter_from_kind("mosaic"),
        Some(Filter::Mosaic { cell_size: 10 })
    );
    assert_eq!(
        filter_from_kind("crystallize"),
        Some(Filter::Crystallize {
            cell_size: 10,
            seed: 1,
        })
    );
    assert_eq!(filter_from_kind("facet"), Some(Filter::Facet));
    assert_eq!(filter_from_kind("fragment"), Some(Filter::Fragment));
    assert_eq!(
        filter_from_kind("mezzotint"),
        Some(Filter::Mezzotint {
            kind: MezzotintType::FineDots,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("pointillize"),
        Some(Filter::Pointillize {
            cell_size: 5,
            background: [0, 0, 0],
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("color-halftone"),
        Some(Filter::ColorHalftone {
            max_radius: 5,
            angles: [108.0, 162.0, 90.0, 45.0],
        })
    );
    assert_eq!(
        filter_from_kind("twirl"),
        Some(Filter::Twirl { angle: 90.0 })
    );
    assert_eq!(
        filter_from_kind("pinch"),
        Some(Filter::Pinch { amount: 50.0 })
    );
    assert_eq!(
        filter_from_kind("spherize"),
        Some(Filter::Spherize {
            amount: 100.0,
            mode: SpherizeMode::Normal,
        })
    );
    assert_eq!(
        filter_from_kind("ripple"),
        Some(Filter::Ripple {
            amount: 100.0,
            size: RippleSize::Medium,
        })
    );
    assert_eq!(
        filter_from_kind("wave"),
        Some(Filter::Wave {
            generators: 5,
            wavelength: (10.0, 120.0),
            amplitude: (5.0, 35.0),
            kind: WaveType::Sine,
            scale: (100.0, 100.0),
            seed: 1,
            repeat_edge: true,
        })
    );
    assert_eq!(
        filter_from_kind("polar-coordinates"),
        Some(Filter::PolarCoordinates {
            kind: PolarKind::RectangularToPolar,
        })
    );
    assert_eq!(
        filter_from_kind("shear"),
        Some(Filter::Shear {
            curve: vec![(-1.0, 0.0), (1.0, 0.0)],
            fill: ShearFill::WrapAround,
        })
    );
    assert_eq!(
        filter_from_kind("zigzag"),
        Some(Filter::ZigZag {
            amount: 50.0,
            ridges: 5,
            style: ZigZagStyle::AroundCenter,
        })
    );
    assert_eq!(
        filter_from_kind("ocean-ripple"),
        Some(Filter::OceanRipple {
            size: 9,
            magnitude: 9,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("clouds"),
        Some(Filter::Clouds {
            color_a: [0, 0, 0],
            color_b: [255, 255, 255],
            starker: false,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("difference-clouds"),
        Some(Filter::DifferenceClouds {
            color_a: [0, 0, 0],
            color_b: [255, 255, 255],
            starker: false,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("fibers"),
        Some(Filter::Fibers {
            variance: 16.0,
            strength: 4.0,
            color_a: [0, 0, 0],
            color_b: [255, 255, 255],
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("lens-flare"),
        Some(Filter::LensFlare {
            brightness: 100.0,
            center: (0.5, 0.5),
            lens: LensType::Zoom,
        })
    );
    assert_eq!(
        filter_from_kind("picture-frame"),
        Some(Filter::PictureFrame {
            options: pictura_filters::FrameOptions::default(),
        })
    );
    assert_eq!(
        filter_from_kind("colored-pencil"),
        Some(Filter::ColoredPencil {
            pencil_width: 4,
            stroke_pressure: 8,
            paper_brightness: 25,
            background: [255, 255, 255],
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("cutout"),
        Some(Filter::Cutout {
            levels: 4,
            edge_simplicity: 4,
            edge_fidelity: 2,
        })
    );
    assert_eq!(
        filter_from_kind("dry-brush"),
        Some(Filter::DryBrush {
            brush_size: 2,
            brush_detail: 8,
            texture: 1,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("film-grain"),
        Some(Filter::FilmGrain {
            grain: 4,
            highlight_area: 0,
            intensity: 10,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("fresco"),
        Some(Filter::Fresco {
            brush_size: 2,
            brush_detail: 8,
            texture: 1,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("neon-glow"),
        Some(Filter::NeonGlow {
            glow_size: 5,
            glow_brightness: 15,
            glow_color: [0, 0, 255],
        })
    );
    assert_eq!(
        filter_from_kind("paint-daubs"),
        Some(Filter::PaintDaubs {
            brush_size: 8,
            sharpness: 7,
            brush_type: BrushType::Simple,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("palette-knife"),
        Some(Filter::PaletteKnife {
            stroke_size: 25,
            stroke_detail: 3,
            softness: 0,
        })
    );
    assert_eq!(
        filter_from_kind("plastic-wrap"),
        Some(Filter::PlasticWrap {
            highlight_strength: 15,
            detail: 9,
            smoothness: 7,
        })
    );
    assert_eq!(
        filter_from_kind("poster-edges"),
        Some(Filter::PosterEdges {
            edge_thickness: 2,
            edge_intensity: 1,
            posterization: 2,
        })
    );
    assert_eq!(
        filter_from_kind("rough-pastels"),
        Some(Filter::RoughPastels {
            stroke_length: 6,
            stroke_detail: 4,
            texture: TextureOptions {
                relief: 20,
                ..TextureOptions::default()
            },
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("smudge-stick"),
        Some(Filter::SmudgeStick {
            stroke_length: 2,
            highlight_area: 0,
            intensity: 10,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("sponge"),
        Some(Filter::Sponge {
            brush_size: 2,
            definition: 12,
            smoothness: 5,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("underpainting"),
        Some(Filter::Underpainting {
            brush_size: 6,
            texture_coverage: 16,
            texture: TextureOptions {
                light_direction: 4,
                ..TextureOptions::default()
            },
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("watercolor"),
        Some(Filter::Watercolor {
            brush_detail: 9,
            shadow_intensity: 1,
            texture: 1,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind("dust-and-scratches"),
        Some(Filter::DustAndScratches {
            radius: 1,
            threshold: 0,
        })
    );
    assert_eq!(
        filter_from_kind("extrude"),
        Some(Filter::Extrude {
            kind: ExtrudeType::Blocks,
            size: 30,
            depth: 30.0,
            level_based: true,
            solid_front: false,
            mask_incomplete: false,
        })
    );
    assert_eq!(
        filter_from_kind("tiles"),
        Some(Filter::Tiles {
            count: 10,
            offset: 10,
            fill: TileFill::BackgroundColor,
            foreground: [0, 0, 0],
            background: [255, 255, 255],
        })
    );
    assert_eq!(
        filter_from_kind("trace-contour"),
        Some(Filter::TraceContour {
            level: 128,
            edge: ContourEdge::Lower,
        })
    );
    assert_eq!(
        filter_from_kind("wind"),
        Some(Filter::Wind {
            method: WindMethod::Wind,
            from_right: true,
        })
    );
    assert_eq!(
        filter_from_kind("smart-sharpen"),
        Some(Filter::SmartSharpen {
            amount: 100.0,
            radius: 1.0,
            reduce_noise: 0.0,
            remove: SharpenRemove::GaussianBlur,
            angle: 0.0,
            more_accurate: false,
            shadow: TonalFade::default(),
            highlight: TonalFade::default(),
        })
    );
    assert_eq!(filter_from_kind("bogus"), None);
}

#[test]
fn filter_from_kind_maps_family_kinds() {
    const FILTER_FAMILY_KINDS: [&str; 29] = [
        "accented-edges",
        "angled-strokes",
        "crosshatch",
        "dark-strokes",
        "ink-outlines",
        "spatter",
        "sprayed-strokes",
        "sumi-e",
        "bas-relief",
        "chalk-charcoal",
        "charcoal",
        "chrome",
        "conte-crayon",
        "graphic-pen",
        "halftone-pattern",
        "note-paper",
        "photocopy",
        "plaster",
        "reticulation",
        "stamp",
        "torn-edges",
        "water-paper",
        "craquelure",
        "grain",
        "mosaic-tiles",
        "patchwork",
        "stained-glass",
        "texturizer",
        "oil-paint",
    ];
    for kind in FILTER_FAMILY_KINDS {
        assert!(filter_from_kind(kind).is_some(), "{kind} should map");
    }
    assert_eq!(filter_from_kind("bogus"), None);
}

#[test]
fn filter_confines_to_selection_and_skips_adjustment_layer() {
    let mut doc = Document::new(8, 1, ColorMode::Rgb, BitDepth::Eight);
    let mut base = pixel_layer("base", 8, 1, (40, 40, 40));
    for ch in base.channels.iter_mut().filter(|c| c.id >= 0) {
        ch.data = (0..8).map(|x| if x < 4 { 40u8 } else { 200 }).collect();
    }
    // A topmost adjustment must be skipped in favour of the pixel layer.
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        base,
        adjustment_layer("invert", None).expect("known kind"),
    ]);

    let selection = Selection {
        width: 8,
        height: 1,
        data: vec![255, 255, 255, 0, 0, 0, 0, 0],
    };
    let mask = selection_to_mask(&selection, &doc);
    let layer = active_pixel_layer_mut(&mut doc, Some("0")).expect("pixel layer");
    assert_eq!(layer.name, "base");
    let before = layer
        .channels
        .iter()
        .find(|c| c.id == 0)
        .expect("red channel")
        .data
        .clone();

    pictura_render::apply_filter(
        layer,
        &filter_from_kind("gaussian-blur").expect("known kind"),
        Some(&mask),
        false,
    )
    .expect("filter applies");

    let after = &layer.channels.iter().find(|c| c.id == 0).unwrap().data;
    assert!(
        after[..4].iter().zip(&before[..4]).any(|(a, b)| a != b),
        "selected pixels should change"
    );
    assert_eq!(&after[4..], &before[4..], "unselected pixels changed");
}

#[test]
fn parse_resample_maps_known_and_rejects_unknown() {
    use pictura_render::Resample;

    assert_eq!(parse_resample("nearest"), Some(Resample::Nearest));
    assert_eq!(parse_resample("bilinear"), Some(Resample::Bilinear));
    assert_eq!(parse_resample("bicubic"), Some(Resample::Bicubic));
    assert_eq!(parse_resample("Bilinear"), None);
    assert_eq!(parse_resample(""), None);
    assert_eq!(parse_resample("gaussian"), None);
}

#[test]
fn parse_anchor_maps_known_and_rejects_unknown() {
    use pictura_render::Anchor;

    assert_eq!(parse_anchor("top-left"), Some(Anchor::TopLeft));
    assert_eq!(parse_anchor("top-center"), Some(Anchor::TopCenter));
    assert_eq!(parse_anchor("top-right"), Some(Anchor::TopRight));
    assert_eq!(parse_anchor("center-left"), Some(Anchor::MiddleLeft));
    assert_eq!(parse_anchor("center"), Some(Anchor::Center));
    assert_eq!(parse_anchor("center-right"), Some(Anchor::MiddleRight));
    assert_eq!(parse_anchor("bottom-left"), Some(Anchor::BottomLeft));
    assert_eq!(parse_anchor("bottom-center"), Some(Anchor::BottomCenter));
    assert_eq!(parse_anchor("bottom-right"), Some(Anchor::BottomRight));
    assert_eq!(parse_anchor("top"), None);
    assert_eq!(parse_anchor("middle"), None);
    assert_eq!(parse_anchor(""), None);
}

#[test]
fn document_ops_wire_parsed_values_and_reject_invalid_params() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![pixel_layer("base", 2, 1, (0, 0, 0))]);
    doc.layers[0].channels = vec![
        Channel {
            id: 0,
            data: vec![255, 0].into(),
        },
        Channel {
            id: 1,
            data: vec![0, 0].into(),
        },
        Channel {
            id: 2,
            data: vec![0, 255].into(),
        },
        Channel {
            id: -1,
            data: vec![255, 255].into(),
        },
    ];
    pictura_render::flip_document(&mut doc, true);
    doc.composite = pictura_render::composite_rgba(&doc);
    let plane = 2usize;
    assert_eq!(
        [
            doc.composite.data[0],
            doc.composite.data[plane],
            doc.composite.data[2 * plane]
        ],
        [0, 0, 255],
        "left pixel mirrors to the old right pixel"
    );
    assert_eq!(
        [
            doc.composite.data[1],
            doc.composite.data[plane + 1],
            doc.composite.data[2 * plane + 1]
        ],
        [255, 0, 0],
        "right pixel mirrors to the old left pixel"
    );

    pictura_render::rotate_document(&mut doc, 1).expect("valid rotate");
    assert_eq!((doc.width, doc.height), (1, 2));

    pictura_render::resize_document(&mut doc, 4, 4, parse_resample("bicubic").unwrap())
        .expect("valid resize");
    assert_eq!((doc.width, doc.height), (4, 4));

    let snapshot = doc.composite.data.clone();
    assert!(
        pictura_render::resize_document(&mut doc, 0, 4, parse_resample("nearest").unwrap())
            .is_err()
    );
    assert_eq!((doc.width, doc.height), (4, 4));
    assert_eq!(
        doc.composite.data, snapshot,
        "failed resize must not mutate"
    );
}

/// Prints the per-call cost of one full-document composite. Prints only; no
/// pass/fail budget, because the reference machine is not pinned.
#[test]
fn composite_rgba_timing_1024() {
    let mut doc = Document::new(1024, 1024, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("base", 1024, 1024, (30, 60, 90)),
        pixel_layer("top", 1024, 1024, (200, 100, 50)),
    ]);
    let _ = pictura_render::composite_rgba(&doc);
    const ITERS: u32 = 5;
    let start = std::time::Instant::now();
    for _ in 0..ITERS {
        let _ = pictura_render::composite_rgba(&doc);
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(ITERS);
    println!("composite_rgba 1024x1024 (2 layers): {ms:.2} ms/call");
    assert_eq!(pictura_render::composite_rgba(&doc).width, 1024);
}

/// Print-only timing breakdown of the Move tool path. Prints
/// `move_profile <name> (<n>x<n>): <ms> ms` lines and no pass/fail budget.
///
/// The real `PictureView` is a C++-constructed QObject (cxx-qt 0.10 exposes
/// no Rust constructor), so the two bridge rows replay the exact bodies of
/// `begin_move_preview`/`commit_move` against the same private helpers. The
/// live QObject is timed separately in the C++ self-test.
fn move_profile(n: u32) {
    let ms = |label: &str, d: std::time::Duration| {
        println!(
            "move_profile {label} ({n}x{n}): {:.2} ms",
            d.as_secs_f64() * 1000.0
        );
    };

    let mut doc = Document::new(n, n, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data.fill(255);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("base", n, n, (255, 255, 255)),
        pixel_layer("top", n, n, (200, 100, 50)),
    ]);

    // Prime the GPU adapter OnceLock and the allocator once, untimed.
    let (_, backend) = pictura_render::composite_active(&doc, true);

    let t = std::time::Instant::now();
    let clone = doc.clone();
    ms("Document::clone", t.elapsed());
    drop(clone);

    let mut moved = doc.clone();
    let t = std::time::Instant::now();
    let translated = pictura_render::translate_layer(&mut moved, 10, 10);
    ms("translate_layer(no composite)", t.elapsed());
    assert!(translated);

    let t = std::time::Instant::now();
    let _ = pictura_render::composite_rgba(&doc);
    ms("composite_rgba(CPU)", t.elapsed());

    let t = std::time::Instant::now();
    let _ = pictura_render::composite_active(&doc, true);
    ms("composite_active(gpu=true)", t.elapsed());

    let t = std::time::Instant::now();
    let _ = pictura_render::composite_active(&doc, false);
    ms("composite_active(gpu=false)", t.elapsed());

    // Reporter's claim: moving the layer outside the canvas should shrink
    // the clamped-rect blend work. Push the top layer fully off-canvas.
    let off = |r: PsdRect| PsdRect {
        top: r.top + n as i32 + 10,
        left: r.left + n as i32 + 10,
        bottom: r.bottom + n as i32 + 10,
        right: r.right + n as i32 + 10,
    };
    let mut off_top = doc.clone();
    off_top.layers[1].rect = off(off_top.layers[1].rect);
    let t = std::time::Instant::now();
    let _ = pictura_render::composite_rgba(&off_top);
    ms("composite_rgba(top off-canvas)", t.elapsed());

    let mut off_all = doc.clone();
    for layer in &mut off_all.layers {
        layer.rect = off(layer.rect);
    }
    let t = std::time::Instant::now();
    let _ = pictura_render::composite_rgba(&off_all);
    ms("composite_rgba(all off-canvas)", t.elapsed());

    if let Err(err) = pictura_render::composite_gpu(&doc) {
        println!("move_profile composite_gpu refused: {err}");
    }

    // 4-channel composite, as the app transaction leaves it for the commit path.
    let t = std::time::Instant::now();
    let _ = buffer_to_image(&moved.composite);
    ms("buffer_to_image", t.elapsed());

    let t = std::time::Instant::now();
    let mut history = History::default();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Move Layer",
    );
    ms("History::capture(clone)", t.elapsed());

    let t = std::time::Instant::now();
    let thumbnail = layer_thumbnail_image(&doc.layers[1], 24);
    ms("layer_thumbnail(24)", t.elapsed());
    assert!(thumbnail.is_some());

    // sample_argb before this change re-composited the whole document per
    // call; after, it reads the already-current cached image.
    let t = std::time::Instant::now();
    let _ = current_buffer(&doc, true);
    ms("sample_argb(before: composite)", t.elapsed());
    let cached = buffer_to_image(&doc.composite);
    let t = std::time::Instant::now();
    let _ = cached.pixel_color(0, 0);
    ms("sample_argb(after: cached)", t.elapsed());

    // begin_move_preview (M35 body): clone the authoritative planar
    // composite and patch only the moved layer's clamped rect with the
    // region composited with the layer hidden.
    let mut preview = doc.clone();
    let t = std::time::Instant::now();
    let index = topmost_pixel_layer_index(&preview).expect("pixel layer");
    let _ = layer_image(&preview.layers[index]);
    let preview_rect = preview.layers[index].rect;
    let preview_ok = preview.composite.width == preview.width
        && preview.composite.height == preview.height
        && !preview.composite.data.is_empty();
    let preview_base =
        match move_preview_region(preview_rect, preview.width, preview.height, preview_ok) {
            Some((x0, y0, ..)) => {
                preview.layers[index].visible = false;
                let (buffer, _backend) =
                    pictura_render::composite_region_active(&preview, preview_rect, true);
                preview.layers[index].visible = true;
                let mut base_buffer = preview.composite.clone();
                patch_buffer_region(&mut base_buffer, &buffer, x0, y0);
                buffer_to_image(&base_buffer)
            }
            None => {
                preview.layers[index].visible = false;
                let base = document_to_image(&preview, true);
                preview.layers[index].visible = true;
                base
            }
        };
    ms("begin_move_preview(region body, full layer)", t.elapsed());
    drop(preview_base);

    // begin_move_preview with the moved layer covering only a sub-rectangle.
    let mut partial = Document::new(n, n, ColorMode::Rgb, BitDepth::Eight);
    partial.composite.data.fill(255);
    partial.layers = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("base", n, n, (255, 255, 255)),
        pixel_layer("top", 512, 512, (200, 100, 50)),
    ]);
    let rendered = current_buffer(&partial, true);
    store_composite(&mut partial, &rendered);
    let mut partial_preview = partial.clone();
    let t = std::time::Instant::now();
    let pindex = topmost_pixel_layer_index(&partial_preview).expect("pixel layer");
    let _ = layer_image(&partial_preview.layers[pindex]);
    let partial_rect = partial_preview.layers[pindex].rect;
    let partial_ok = partial_preview.composite.width == partial_preview.width
        && partial_preview.composite.height == partial_preview.height
        && !partial_preview.composite.data.is_empty();
    let partial_base = match move_preview_region(
        partial_rect,
        partial_preview.width,
        partial_preview.height,
        partial_ok,
    ) {
        Some((x0, y0, ..)) => {
            partial_preview.layers[pindex].visible = false;
            let (buffer, _backend) =
                pictura_render::composite_region_active(&partial_preview, partial_rect, true);
            partial_preview.layers[pindex].visible = true;
            let mut base_buffer = partial_preview.composite.clone();
            patch_buffer_region(&mut base_buffer, &buffer, x0, y0);
            buffer_to_image(&base_buffer)
        }
        None => {
            partial_preview.layers[pindex].visible = false;
            let base = document_to_image(&partial_preview, true);
            partial_preview.layers[pindex].visible = true;
            base
        }
    };
    ms(
        "begin_move_preview(region body, partial 512x512 layer)",
        t.elapsed(),
    );
    drop(partial_base);

    // begin_move_preview cache hit: find the topmost layer, clamp its rect,
    // and refresh the cached origin/opacity. No region composite, no readback.
    let t = std::time::Instant::now();
    let hit_index = topmost_pixel_layer_index(&partial_preview).expect("pixel layer");
    let hit_rect = partial_preview.layers[hit_index].rect;
    let _hit_clamped = (
        hit_rect.left.max(0),
        hit_rect.top.max(0),
        hit_rect.right.min(partial_preview.width as i32),
        hit_rect.bottom.min(partial_preview.height as i32),
    );
    let _hit_origin = (
        hit_rect.left,
        hit_rect.top,
        partial_preview.layers[hit_index].opacity,
    );
    ms("begin_move_preview(cache hit)", t.elapsed());

    let mut committed = doc.clone();
    let t = std::time::Instant::now();
    let committed_ok = pictura_render::translate_layer_active(&mut committed, 50, 50, true);
    let mut history = History::default();
    history.capture(
        Snapshot {
            doc: committed.clone(),
            selection: None,
        },
        "Move Layer",
    );
    let _ = buffer_to_image(&committed.composite);
    ms("commit_move(bridge-equivalent)", t.elapsed());
    assert!(committed_ok);

    println!("move_profile composite_active backend: {backend:?}");
}

#[test]
#[ignore = "4000x4000 profile; run explicitly with --ignored --nocapture"]
fn move_profile_4000() {
    move_profile(4000);
}

#[test]
#[ignore = "large-document profile; run explicitly with --ignored --nocapture"]
fn move_profile_1024() {
    move_profile(1024);
}

/// Print-only evidence that the M34 undo/redo display no longer recomposites.
/// Prints `m34 undo_profile ...` lines and no pass/fail budget, because the
/// reference machine is not pinned. The real `PictureView::undo` is a
/// C++-constructed QObject, so this replays the exact display body
/// (`buffer_to_image(&snapshot.doc.composite)`) against the same helpers.
#[test]
#[ignore = "4000x4000 undo profile; run explicitly with --ignored --nocapture"]
fn undo_profile_4000() {
    let ms = |label: &str, d: std::time::Duration| {
        println!(
            "undo_profile {label} 4000x4000: {:.2} ms",
            d.as_secs_f64() * 1000.0
        );
    };

    let mut doc = Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("base", 4000, 4000, (30, 60, 90)),
        pixel_layer("top", 4000, 4000, (200, 100, 50)),
    ]);

    // Prime the GPU adapter and allocator once, untimed.
    let (_, backend) = pictura_render::composite_active(&doc, true);

    // Old undo display path: full composite + buffer_to_image.
    let t = std::time::Instant::now();
    let _ = document_to_image(&doc, true);
    ms("old document_to_image(gpu=true)", t.elapsed());

    // Persist the composite the way the bridge's `recomposite` does.
    let rendered = current_buffer(&doc, true);
    store_composite(&mut doc, &rendered);

    // New undo display path: the snapshot already holds the composite.
    let t = std::time::Instant::now();
    let _ = buffer_to_image(&doc.composite);
    ms("new buffer_to_image(snapshot.composite)", t.elapsed());

    // The remaining per-snapshot undo cost: the whole-document clone.
    let t = std::time::Instant::now();
    let mut history = History::default();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Undo Profile",
    );
    ms("History::capture(clone)", t.elapsed());

    println!("undo_profile composite_active backend: {backend:?}");
}

/// The bridge QObject is C++-constructed (cxx-qt 0.10 exposes no Rust
/// constructor), so this replays the public `convert_profile(1)` + `undo()`
/// sequence against the same codec and `History` and checks the restored bytes.
#[test]
fn convert_profile_is_byte_reversible_through_history() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = vec![200, 10, 100, 20, 50, 30].into();
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("base", 2, 1, (200, 100, 50))]);
    let original = doc.clone();
    let mut history = History::default();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Open",
    );

    assert!(pictura_codec::convert_document(
        &mut doc,
        &pictura_codec::Profile::adobe_rgb()
    ));
    let adobe = Some(pictura_codec::Profile::adobe_rgb().to_icc());
    assert_eq!(doc.document_icc, adobe);
    assert_ne!(
        doc.composite, original.composite,
        "convert changes the composite"
    );
    assert_ne!(
        doc.layers, original.layers,
        "convert changes the layer pixels"
    );
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Convert to Profile",
    );

    assert_eq!(history.depth(), 1, "exactly one undo step");
    let restored = history.undo().expect("one undo step");
    assert_eq!(restored.doc.composite, original.composite);
    assert_eq!(restored.doc.layers, original.layers);
    assert_eq!(restored.doc.document_icc, original.document_icc);
    assert!(restored.doc.document_icc.is_none());
}

/// Bridge-equivalent byte-level check for `assign_profile(1)` + `undo()`:
/// assign leaves every pixel byte untouched, then undo restores the working
/// profile and the image-resource section exactly.
#[test]
fn assign_profile_retags_and_is_reversible_through_history() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = vec![200, 10, 100, 20, 50, 30].into();
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("base", 2, 1, (200, 100, 50))]);
    let original = doc.clone();
    let mut history = History::default();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Open",
    );

    pictura_codec::assign_document_profile(&mut doc, &pictura_codec::Profile::adobe_rgb());
    let adobe = Some(pictura_codec::Profile::adobe_rgb().to_icc());
    assert_eq!(
        doc.composite, original.composite,
        "assign leaves the composite bytes untouched"
    );
    assert_eq!(
        doc.layers, original.layers,
        "assign leaves the layer bytes untouched"
    );
    assert_eq!(doc.document_icc, adobe);
    assert_ne!(
        doc.image_resources, original.image_resources,
        "assign writes resource 1039"
    );
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Assign Profile",
    );

    assert_eq!(history.depth(), 1, "exactly one undo step");
    let restored = history.undo().expect("one undo step");
    assert_eq!(restored.doc.composite, original.composite);
    assert_eq!(restored.doc.layers, original.layers);
    assert_eq!(restored.doc.document_icc, original.document_icc);
    assert_eq!(restored.doc.image_resources, original.image_resources);
}

/// The first frame `open` builds (and `File > Revert`, which reuses `open`) must
/// show the working-space conversion, not the raw embedded-space composite: an
/// Adobe RGB document displays through `buffer_to_srgb`, so its first frame
/// differs from the untagged one.
#[test]
fn open_first_frame_converts_the_document_profile() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = vec![200, 10, 100, 20, 50, 30].into();
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("base", 2, 1, (200, 100, 50))]);
    doc.document_icc = Some(pictura_codec::Profile::adobe_rgb().to_icc());

    let rendered = current_buffer(&doc, false);
    let converted = pictura_codec::buffer_to_srgb(&doc, &rendered);
    assert_ne!(
        *converted, rendered,
        "an Adobe RGB document must be converted for display"
    );
    let first_frame = buffer_to_image(&converted);
    let raw_frame = buffer_to_image(&rendered);
    assert_ne!(
        first_frame.pixel_color(0, 0),
        raw_frame.pixel_color(0, 0),
        "the displayed first frame must not be the embedded-space pixels"
    );
}
