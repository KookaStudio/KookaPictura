//! The shared filter contract for every variant `apply` routes to the ported
//! photorust engine (#221, #222): it runs on RGB and RGBA, leaves alpha alone
//! for value kernels and filters it for moving ones (see
//! [`pictura_filters::Filter::moves_alpha`]), is deterministic, rejects an
//! out-of-range parameter without writing a sample, survives tiny images, and
//! re-rolls with its seed where it takes one.

use std::sync::Arc;

use pictura_core::PixelBuffer;
use pictura_filters::*;

const W: u32 = 24;
const H: u32 = 20;

/// A deterministic picture with gradients, hard edges, and varied alpha.
fn picture(channels: u8) -> PixelBuffer {
    let mut b = PixelBuffer::new(W, H, channels);
    let n = (W * H) as usize;
    for y in 0..H as usize {
        for x in 0..W as usize {
            let i = y * W as usize + x;
            b.data[i] = (x * 10) as u8;
            b.data[n + i] = (y * 12) as u8;
            b.data[2 * n + i] = if (x / 4 + y / 4) % 2 == 0 { 40 } else { 220 };
            if channels == 4 {
                b.data[3 * n + i] = (100 + x * 6) as u8;
            }
        }
    }
    b
}

fn tex() -> TextureOptions {
    TextureOptions::default()
}

/// One row per ported filter: a valid instance, an out-of-range instance
/// (`None` when every value of its type is valid), and whether it is seeded.
struct Case {
    name: &'static str,
    valid: Filter,
    invalid: Option<Filter>,
    seeded: Option<fn(u64) -> Filter>,
}

fn case(name: &'static str, valid: Filter, invalid: Option<Filter>) -> Case {
    Case {
        name,
        valid,
        invalid,
        seeded: None,
    }
}

fn seeded(name: &'static str, invalid: Option<Filter>, make: fn(u64) -> Filter) -> Case {
    Case {
        name,
        valid: make(7),
        invalid,
        seeded: Some(make),
    }
}

#[allow(clippy::too_many_lines)]
fn cases() -> Vec<Case> {
    let fg = [20, 30, 40];
    let bg = [230, 220, 210];
    vec![
        seeded(
            "AddNoise",
            Some(Filter::AddNoise {
                amount: 401.0,
                distribution: NoiseDistribution::Uniform,
                monochromatic: false,
                seed: 1,
            }),
            |seed| Filter::AddNoise {
                amount: 40.0,
                distribution: NoiseDistribution::Gaussian,
                monochromatic: false,
                seed,
            },
        ),
        case(
            "DustAndScratches",
            Filter::DustAndScratches {
                radius: 2,
                threshold: 10,
            },
            Some(Filter::DustAndScratches {
                radius: 17,
                threshold: 10,
            }),
        ),
        case("Average", Filter::Average, None),
        case(
            "SurfaceBlur",
            Filter::SurfaceBlur {
                radius: 3,
                threshold: 30,
            },
            Some(Filter::SurfaceBlur {
                radius: 101,
                threshold: 30,
            }),
        ),
        case("SharpenEdges", Filter::SharpenEdges, None),
        case(
            "Emboss",
            Filter::Emboss {
                angle: 135.0,
                height: 3.0,
                amount: 100.0,
            },
            Some(Filter::Emboss {
                angle: 400.0,
                height: 3.0,
                amount: 100.0,
            }),
        ),
        case("FindEdges", Filter::FindEdges, None),
        case(
            "Diffuse",
            Filter::Diffuse {
                mode: DiffuseMode::Normal,
            },
            None,
        ),
        case(
            "GlowingEdges",
            Filter::GlowingEdges {
                width: 2,
                brightness: 6,
                smoothness: 5,
            },
            Some(Filter::GlowingEdges {
                width: 15,
                brightness: 6,
                smoothness: 5,
            }),
        ),
        case(
            "Extrude",
            Filter::Extrude {
                kind: ExtrudeType::Blocks,
                size: 6,
                depth: 10.0,
                level_based: false,
                solid_front: false,
                mask_incomplete: false,
            },
            Some(Filter::Extrude {
                kind: ExtrudeType::Blocks,
                size: 1,
                depth: 10.0,
                level_based: false,
                solid_front: false,
                mask_incomplete: false,
            }),
        ),
        case(
            "Tiles",
            Filter::Tiles {
                count: 4,
                offset: 10,
                fill: TileFill::BackgroundColor,
                foreground: fg,
                background: bg,
            },
            Some(Filter::Tiles {
                count: 0,
                offset: 10,
                fill: TileFill::BackgroundColor,
                foreground: fg,
                background: bg,
            }),
        ),
        case(
            "TraceContour",
            Filter::TraceContour {
                level: 128,
                edge: ContourEdge::Upper,
            },
            None,
        ),
        case(
            "Wind",
            Filter::Wind {
                method: WindMethod::Wind,
                from_right: false,
            },
            None,
        ),
        case(
            "Mosaic",
            Filter::Mosaic { cell_size: 4 },
            Some(Filter::Mosaic { cell_size: 1 }),
        ),
        seeded(
            "Crystallize",
            Some(Filter::Crystallize {
                cell_size: 2,
                seed: 1,
            }),
            |seed| Filter::Crystallize { cell_size: 5, seed },
        ),
        seeded(
            "Pointillize",
            Some(Filter::Pointillize {
                cell_size: 2,
                background: [255; 3],
                seed: 1,
            }),
            |seed| Filter::Pointillize {
                cell_size: 4,
                background: [255; 3],
                seed,
            },
        ),
        case("Facet", Filter::Facet, None),
        case("Fragment", Filter::Fragment, None),
        seeded("Mezzotint", None, |seed| Filter::Mezzotint {
            kind: MezzotintType::MediumDots,
            seed,
        }),
        case(
            "ColorHalftone",
            Filter::ColorHalftone {
                max_radius: 6,
                angles: [108.0, 162.0, 90.0, 45.0],
            },
            Some(Filter::ColorHalftone {
                max_radius: 3,
                angles: [108.0, 162.0, 90.0, 45.0],
            }),
        ),
        case(
            "Twirl",
            Filter::Twirl { angle: 50.0 },
            Some(Filter::Twirl { angle: 1000.0 }),
        ),
        case(
            "Pinch",
            Filter::Pinch { amount: 50.0 },
            Some(Filter::Pinch { amount: 101.0 }),
        ),
        case(
            "Spherize",
            Filter::Spherize {
                amount: 50.0,
                mode: SpherizeMode::Normal,
            },
            Some(Filter::Spherize {
                amount: f64::NAN,
                mode: SpherizeMode::Normal,
            }),
        ),
        case(
            "Ripple",
            Filter::Ripple {
                amount: 100.0,
                size: RippleSize::Medium,
            },
            Some(Filter::Ripple {
                amount: 1000.0,
                size: RippleSize::Medium,
            }),
        ),
        case(
            "PolarCoordinates",
            Filter::PolarCoordinates {
                kind: PolarKind::RectangularToPolar,
            },
            None,
        ),
        case(
            "ZigZag",
            Filter::ZigZag {
                amount: 20.0,
                ridges: 5,
                style: ZigZagStyle::PondRipples,
            },
            Some(Filter::ZigZag {
                amount: 20.0,
                ridges: 21,
                style: ZigZagStyle::PondRipples,
            }),
        ),
        seeded(
            "Wave",
            Some(Filter::Wave {
                generators: 0,
                wavelength: (10.0, 120.0),
                amplitude: (5.0, 35.0),
                kind: WaveType::Sine,
                scale: (100.0, 100.0),
                seed: 1,
                repeat_edge: false,
            }),
            |seed| Filter::Wave {
                generators: 5,
                wavelength: (10.0, 120.0),
                amplitude: (5.0, 35.0),
                kind: WaveType::Sine,
                scale: (100.0, 100.0),
                seed,
                repeat_edge: false,
            },
        ),
        seeded(
            "ColoredPencil",
            Some(Filter::ColoredPencil {
                pencil_width: 25,
                stroke_pressure: 8,
                paper_brightness: 25,
                background: bg,
                seed: 1,
            }),
            |seed| Filter::ColoredPencil {
                pencil_width: 4,
                stroke_pressure: 8,
                paper_brightness: 25,
                background: [230, 220, 210],
                seed,
            },
        ),
        case(
            "Cutout",
            Filter::Cutout {
                levels: 4,
                edge_simplicity: 4,
                edge_fidelity: 2,
            },
            Some(Filter::Cutout {
                levels: 9,
                edge_simplicity: 4,
                edge_fidelity: 2,
            }),
        ),
        seeded(
            "DryBrush",
            Some(Filter::DryBrush {
                brush_size: 11,
                brush_detail: 8,
                texture: 1,
                seed: 1,
            }),
            |seed| Filter::DryBrush {
                brush_size: 2,
                brush_detail: 8,
                texture: 1,
                seed,
            },
        ),
        seeded(
            "Fresco",
            Some(Filter::Fresco {
                brush_size: 2,
                brush_detail: 13,
                texture: 1,
                seed: 1,
            }),
            |seed| Filter::Fresco {
                brush_size: 2,
                brush_detail: 8,
                texture: 1,
                seed,
            },
        ),
        seeded(
            "FilmGrain",
            Some(Filter::FilmGrain {
                grain: 21,
                highlight_area: 0,
                intensity: 10,
                seed: 1,
            }),
            |seed| Filter::FilmGrain {
                grain: 4,
                highlight_area: 0,
                intensity: 10,
                seed,
            },
        ),
        case(
            "NeonGlow",
            Filter::NeonGlow {
                glow_size: 5,
                glow_brightness: 15,
                glow_color: [0, 120, 255],
            },
            Some(Filter::NeonGlow {
                glow_size: 25,
                glow_brightness: 15,
                glow_color: [0, 120, 255],
            }),
        ),
        seeded(
            "PaintDaubs",
            Some(Filter::PaintDaubs {
                brush_size: 0,
                sharpness: 7,
                brush_type: BrushType::Simple,
                seed: 1,
            }),
            |seed| Filter::PaintDaubs {
                brush_size: 4,
                sharpness: 7,
                brush_type: BrushType::Sparkle,
                seed,
            },
        ),
        case(
            "PaletteKnife",
            Filter::PaletteKnife {
                stroke_size: 6,
                stroke_detail: 3,
                softness: 0,
            },
            Some(Filter::PaletteKnife {
                stroke_size: 25,
                stroke_detail: 4,
                softness: 0,
            }),
        ),
        case(
            "PlasticWrap",
            Filter::PlasticWrap {
                highlight_strength: 15,
                detail: 9,
                smoothness: 7,
            },
            Some(Filter::PlasticWrap {
                highlight_strength: 21,
                detail: 9,
                smoothness: 7,
            }),
        ),
        case(
            "PosterEdges",
            Filter::PosterEdges {
                edge_thickness: 2,
                edge_intensity: 1,
                posterization: 2,
            },
            Some(Filter::PosterEdges {
                edge_thickness: 11,
                edge_intensity: 1,
                posterization: 2,
            }),
        ),
        seeded(
            "RoughPastels",
            Some(Filter::RoughPastels {
                stroke_length: 6,
                stroke_detail: 4,
                texture: TextureOptions {
                    scaling: 49,
                    ..tex()
                },
                seed: 1,
            }),
            |seed| Filter::RoughPastels {
                stroke_length: 6,
                stroke_detail: 4,
                texture: tex(),
                seed,
            },
        ),
        seeded(
            "SmudgeStick",
            Some(Filter::SmudgeStick {
                stroke_length: 11,
                highlight_area: 0,
                intensity: 10,
                seed: 1,
            }),
            |seed| Filter::SmudgeStick {
                stroke_length: 2,
                highlight_area: 0,
                intensity: 10,
                seed,
            },
        ),
        seeded(
            "Sponge",
            Some(Filter::Sponge {
                brush_size: 2,
                definition: 26,
                smoothness: 5,
                seed: 1,
            }),
            |seed| Filter::Sponge {
                brush_size: 2,
                definition: 12,
                smoothness: 5,
                seed,
            },
        ),
        seeded(
            "Underpainting",
            Some(Filter::Underpainting {
                brush_size: 41,
                texture_coverage: 16,
                texture: tex(),
                seed: 1,
            }),
            |seed| Filter::Underpainting {
                brush_size: 6,
                texture_coverage: 16,
                texture: tex(),
                seed,
            },
        ),
        seeded(
            "Watercolor",
            Some(Filter::Watercolor {
                brush_detail: 0,
                shadow_intensity: 1,
                texture: 1,
                seed: 1,
            }),
            |seed| Filter::Watercolor {
                brush_detail: 9,
                shadow_intensity: 1,
                texture: 1,
                seed,
            },
        ),
        case(
            "AccentedEdges",
            Filter::AccentedEdges {
                edge_width: 2,
                edge_brightness: 38,
                smoothness: 5,
            },
            Some(Filter::AccentedEdges {
                edge_width: 0,
                edge_brightness: 38,
                smoothness: 5,
            }),
        ),
        case(
            "AngledStrokes",
            Filter::AngledStrokes {
                direction_balance: 50,
                stroke_length: 15,
                sharpness: 3,
            },
            Some(Filter::AngledStrokes {
                direction_balance: 101,
                stroke_length: 15,
                sharpness: 3,
            }),
        ),
        case(
            "Crosshatch",
            Filter::Crosshatch {
                stroke_length: 9,
                sharpness: 6,
                strength: 1,
            },
            Some(Filter::Crosshatch {
                stroke_length: 9,
                sharpness: 6,
                strength: 4,
            }),
        ),
        case(
            "DarkStrokes",
            Filter::DarkStrokes {
                balance: 5,
                black_intensity: 6,
                white_intensity: 2,
            },
            Some(Filter::DarkStrokes {
                balance: 11,
                black_intensity: 6,
                white_intensity: 2,
            }),
        ),
        case(
            "InkOutlines",
            Filter::InkOutlines {
                stroke_length: 4,
                dark_intensity: 20,
                light_intensity: 10,
            },
            Some(Filter::InkOutlines {
                stroke_length: 0,
                dark_intensity: 20,
                light_intensity: 10,
            }),
        ),
        seeded(
            "Spatter",
            Some(Filter::Spatter {
                spray_radius: 26,
                smoothness: 5,
                seed: 1,
            }),
            |seed| Filter::Spatter {
                spray_radius: 10,
                smoothness: 5,
                seed,
            },
        ),
        seeded(
            "SprayedStrokes",
            Some(Filter::SprayedStrokes {
                stroke_length: 21,
                spray_radius: 7,
                direction: StrokeDirection::RightDiagonal,
                seed: 1,
            }),
            |seed| Filter::SprayedStrokes {
                stroke_length: 12,
                spray_radius: 7,
                direction: StrokeDirection::RightDiagonal,
                seed,
            },
        ),
        case(
            "SumiE",
            Filter::SumiE {
                stroke_width: 10,
                stroke_pressure: 2,
                contrast: 16,
            },
            Some(Filter::SumiE {
                stroke_width: 2,
                stroke_pressure: 2,
                contrast: 16,
            }),
        ),
        case(
            "BasRelief",
            Filter::BasRelief {
                detail: 13,
                smoothness: 3,
                light_direction: LightDirection::Top,
                foreground: fg,
                background: bg,
            },
            Some(Filter::BasRelief {
                detail: 0,
                smoothness: 3,
                light_direction: LightDirection::Top,
                foreground: fg,
                background: bg,
            }),
        ),
        seeded(
            "ChalkCharcoal",
            Some(Filter::ChalkCharcoal {
                charcoal_area: 6,
                chalk_area: 6,
                stroke_pressure: 6,
                foreground: fg,
                background: bg,
                seed: 1,
            }),
            |seed| Filter::ChalkCharcoal {
                charcoal_area: 6,
                chalk_area: 6,
                stroke_pressure: 1,
                foreground: [20, 30, 40],
                background: [230, 220, 210],
                seed,
            },
        ),
        seeded(
            "Charcoal",
            Some(Filter::Charcoal {
                thickness: 8,
                detail: 5,
                light_dark_balance: 50,
                foreground: fg,
                background: bg,
                seed: 1,
            }),
            |seed| Filter::Charcoal {
                thickness: 1,
                detail: 5,
                light_dark_balance: 50,
                foreground: [20, 30, 40],
                background: [230, 220, 210],
                seed,
            },
        ),
        case(
            "Chrome",
            Filter::Chrome {
                detail: 4,
                smoothness: 7,
            },
            Some(Filter::Chrome {
                detail: 11,
                smoothness: 7,
            }),
        ),
        seeded(
            "ConteCrayon",
            Some(Filter::ConteCrayon {
                foreground_level: 0,
                background_level: 7,
                texture: tex(),
                foreground: fg,
                background: bg,
                seed: 1,
            }),
            |seed| Filter::ConteCrayon {
                foreground_level: 11,
                background_level: 7,
                texture: tex(),
                foreground: [20, 30, 40],
                background: [230, 220, 210],
                seed,
            },
        ),
        case(
            "GraphicPen",
            Filter::GraphicPen {
                stroke_length: 15,
                light_dark_balance: 50,
                direction: StrokeDirection::RightDiagonal,
                foreground: fg,
                background: bg,
            },
            Some(Filter::GraphicPen {
                stroke_length: 16,
                light_dark_balance: 50,
                direction: StrokeDirection::RightDiagonal,
                foreground: fg,
                background: bg,
            }),
        ),
        case(
            "HalftonePattern",
            Filter::HalftonePattern {
                size: 1,
                contrast: 5,
                pattern: HalftoneType::Dot,
            },
            Some(Filter::HalftonePattern {
                size: 13,
                contrast: 5,
                pattern: HalftoneType::Dot,
            }),
        ),
        seeded(
            "NotePaper",
            Some(Filter::NotePaper {
                image_balance: 51,
                graininess: 10,
                relief: 11,
                seed: 1,
            }),
            |seed| Filter::NotePaper {
                image_balance: 25,
                graininess: 10,
                relief: 11,
                seed,
            },
        ),
        case(
            "Photocopy",
            Filter::Photocopy {
                detail: 7,
                darkness: 8,
            },
            Some(Filter::Photocopy {
                detail: 7,
                darkness: 0,
            }),
        ),
        case(
            "Plaster",
            Filter::Plaster {
                image_balance: 20,
                smoothness: 2,
                light_direction: LightDirection::Top,
                foreground: fg,
                background: bg,
            },
            Some(Filter::Plaster {
                image_balance: 51,
                smoothness: 2,
                light_direction: LightDirection::Top,
                foreground: fg,
                background: bg,
            }),
        ),
        seeded(
            "Reticulation",
            Some(Filter::Reticulation {
                density: 51,
                black_level: 40,
                white_level: 5,
                foreground: fg,
                background: bg,
                seed: 1,
            }),
            |seed| Filter::Reticulation {
                density: 12,
                black_level: 40,
                white_level: 5,
                foreground: [20, 30, 40],
                background: [230, 220, 210],
                seed,
            },
        ),
        case(
            "Stamp",
            Filter::Stamp {
                light_dark_balance: 25,
                smoothness: 5,
                foreground: fg,
                background: bg,
            },
            Some(Filter::Stamp {
                light_dark_balance: 25,
                smoothness: 0,
                foreground: fg,
                background: bg,
            }),
        ),
        case(
            "TornEdges",
            Filter::TornEdges {
                image_balance: 25,
                smoothness: 11,
                contrast: 17,
                foreground: fg,
                background: bg,
            },
            Some(Filter::TornEdges {
                image_balance: 25,
                smoothness: 11,
                contrast: 26,
                foreground: fg,
                background: bg,
            }),
        ),
        seeded(
            "WaterPaper",
            Some(Filter::WaterPaper {
                fiber_length: 2,
                brightness: 60,
                contrast: 80,
                seed: 1,
            }),
            |seed| Filter::WaterPaper {
                fiber_length: 15,
                brightness: 60,
                contrast: 80,
                seed,
            },
        ),
        case(
            "Craquelure",
            Filter::Craquelure {
                crack_spacing: 15,
                crack_depth: 6,
                crack_brightness: 9,
            },
            Some(Filter::Craquelure {
                crack_spacing: 1,
                crack_depth: 6,
                crack_brightness: 9,
            }),
        ),
        seeded(
            "Grain",
            Some(Filter::Grain {
                intensity: 101,
                contrast: 50,
                grain_type: GrainType::Regular,
                background: bg,
                seed: 1,
            }),
            |seed| Filter::Grain {
                intensity: 40,
                contrast: 50,
                grain_type: GrainType::Regular,
                background: [230, 220, 210],
                seed,
            },
        ),
        seeded(
            "MosaicTiles",
            Some(Filter::MosaicTiles {
                tile_size: 1,
                grout_width: 4,
                lighten_grout: 9,
                seed: 1,
            }),
            |seed| Filter::MosaicTiles {
                tile_size: 6,
                grout_width: 2,
                lighten_grout: 9,
                seed,
            },
        ),
        seeded(
            "Patchwork",
            Some(Filter::Patchwork {
                square_size: 11,
                relief: 8,
                seed: 1,
            }),
            |seed| Filter::Patchwork {
                square_size: 4,
                relief: 8,
                seed,
            },
        ),
        seeded(
            "StainedGlass",
            Some(Filter::StainedGlass {
                cell_size: 1,
                border_thickness: 4,
                light_intensity: 3,
                foreground: fg,
                seed: 1,
            }),
            |seed| Filter::StainedGlass {
                cell_size: 6,
                border_thickness: 2,
                light_intensity: 3,
                foreground: [0, 0, 0],
                seed,
            },
        ),
        case(
            "Texturizer",
            Filter::Texturizer { texture: tex() },
            Some(Filter::Texturizer {
                texture: TextureOptions {
                    light_direction: 8,
                    ..tex()
                },
            }),
        ),
        case(
            "Displace",
            Filter::Displace {
                map: Arc::new(
                    DisplaceMap::new(2, 2, vec![0, 255, 255, 0], vec![255, 0, 0, 255]).unwrap(),
                ),
                horizontal_scale: 10.0,
                vertical_scale: 10.0,
                fit: DisplaceFit::StretchToFit,
                wrap: false,
            },
            Some(Filter::Displace {
                map: Arc::new(DisplaceMap::neutral()),
                horizontal_scale: 1000.0,
                vertical_scale: 10.0,
                fit: DisplaceFit::StretchToFit,
                wrap: false,
            }),
        ),
        case(
            "Shear",
            Filter::Shear {
                curve: vec![(-1.0, -0.5), (0.0, 0.0), (1.0, 0.5)],
                fill: ShearFill::WrapAround,
            },
            Some(Filter::Shear {
                curve: vec![(0.0, 0.0)],
                fill: ShearFill::WrapAround,
            }),
        ),
        case(
            "PictureFrame",
            Filter::PictureFrame {
                options: FrameOptions::default(),
            },
            Some(Filter::PictureFrame {
                options: FrameOptions {
                    frame: 0,
                    ..FrameOptions::default()
                },
            }),
        ),
    ]
}

#[test]
fn ported_cases_cover_every_ported_variant() {
    let cases = cases();
    let covered: std::collections::BTreeSet<&str> = cases.iter().map(|case| case.name).collect();
    let expected: std::collections::BTreeSet<&str> = FilterKind::ALL
        .iter()
        .filter(|kind| kind.engine() == FilterEngine::Ported)
        .map(|kind| kind.name())
        .collect();
    assert_eq!(
        covered, expected,
        "every ported variant needs one contract case"
    );
    assert_eq!(cases.len(), covered.len(), "a ported case is repeated");
}

#[test]
fn every_ported_filter_runs_and_decides_alpha_per_its_kernel() {
    for c in cases() {
        let before = picture(4);
        let mut b = before.clone();
        apply(&c.valid, &mut b).unwrap_or_else(|e| panic!("{}: {e}", c.name));
        let n = (W * H) as usize;
        // A value kernel must leave alpha bit-identical. A moving kernel runs
        // over the fourth plane, but on smooth data (no outliers, no edges) the
        // kernel can be a no-op there and leave the same bytes — layer-level
        // tests in pictura-render pin the filtering behaviour itself.
        if !c.valid.moves_alpha() {
            assert_eq!(
                b.data[3 * n..],
                before.data[3 * n..],
                "{} touched alpha",
                c.name
            );
        }
        // Facet keeps this picture's even gradients and 4-pixel checks as they
        // are; `facet_flattens_a_gradient_and_keeps_a_strong_edge` covers it.
        if c.name != "Facet" {
            assert_ne!(
                b.data[..3 * n],
                before.data[..3 * n],
                "{} left the colour planes as they were",
                c.name
            );
        }
        let mut rgb = picture(3);
        apply(&c.valid, &mut rgb).unwrap_or_else(|e| panic!("{} on RGB: {e}", c.name));
    }
}

#[test]
fn every_ported_filter_is_deterministic() {
    for c in cases() {
        let (mut a, mut b) = (picture(4), picture(4));
        apply(&c.valid, &mut a).unwrap();
        apply(&c.valid, &mut b).unwrap();
        assert_eq!(a.data, b.data, "{} is not deterministic", c.name);
    }
}

#[test]
fn out_of_range_parameters_are_rejected_untouched() {
    for c in cases() {
        let Some(invalid) = c.invalid else { continue };
        let before = picture(4);
        let mut b = before.clone();
        let result = apply(&invalid, &mut b);
        assert!(
            matches!(result, Err(FilterError::InvalidParams(_))),
            "{} accepted an out-of-range parameter: {result:?}",
            c.name
        );
        assert_eq!(b.data, before.data, "{} wrote before rejecting", c.name);
    }
}

/// Values past the documented ranges are rejected, not cast to infinity or
/// wrapped inside the filter.
#[test]
fn huge_parameters_are_rejected_untouched() {
    let wave = |wavelength, amplitude| Filter::Wave {
        generators: 5,
        wavelength,
        amplitude,
        kind: WaveType::Sine,
        scale: (100.0, 100.0),
        seed: 1,
        repeat_edge: false,
    };
    for filter in [
        Filter::ColorHalftone {
            max_radius: 6,
            angles: [1e9, 0.0, 0.0, 0.0],
        },
        Filter::Emboss {
            angle: 0.0,
            height: 1e300,
            amount: 100.0,
        },
        Filter::Emboss {
            angle: 0.0,
            height: 3.0,
            amount: 1e300,
        },
        wave((10.0, 1e300), (5.0, 35.0)),
        wave((10.0, 120.0), (5.0, 1e300)),
    ] {
        let before = picture(4);
        let mut b = before.clone();
        let result = apply(&filter, &mut b);
        assert!(
            matches!(result, Err(FilterError::InvalidParams(_))),
            "{filter:?} accepted: {result:?}"
        );
        assert_eq!(b.data, before.data, "{filter:?} wrote before rejecting");
    }
}

#[test]
fn tiny_images_do_not_panic() {
    for c in cases() {
        for (w, h) in [(1, 1), (1, 9), (9, 1), (2, 2)] {
            let mut b = PixelBuffer::new(w, h, 4);
            b.data.iter_mut().for_each(|v| *v = 200);
            let _ = apply(&c.valid, &mut b);
        }
    }
}

#[test]
fn seeded_filters_re_roll_with_the_seed() {
    let mut fixed = Vec::new();
    for c in cases() {
        let Some(make) = c.seeded else { continue };
        let (mut a, mut b) = (picture(4), picture(4));
        apply(&make(1), &mut a).unwrap();
        apply(&make(2), &mut b).unwrap();
        if a.data == b.data {
            fixed.push(c.name);
        }
    }
    assert!(fixed.is_empty(), "seed has no effect on: {fixed:?}");
}
