//! Print-only CPU profile of `pictura_filters::apply` on a structured
//! 1024x1024 RGB buffer. No assertions, no golden data: it exists to rank the
//! kernels by wall-clock cost and feed the GPU-port decision.

use std::time::Instant;

use pictura_core::PixelBuffer;
use pictura_filters::{
    apply, BrushType, Filter, LensType, LightDirection, MezzotintType, NoiseDistribution,
    PolarKind, Quality, RippleSize, ShearFill, SmartBlurMode, SpherizeMode, TextureOptions,
    WaveType, ZigZagStyle,
};

const SIZE: u32 = 1024;

/// Gradient + two hard edges + a filled circle, so both separable and
/// neighbourhood/order-statistic kernels have real high-frequency work.
fn structured_rgb(size: u32) -> PixelBuffer {
    let mut buf = PixelBuffer::new(size, size, 3);
    let n = (size * size) as usize;
    let w = size as usize;
    let h = size as usize;
    let cx = size as i32 * 3 / 4;
    let cy = size as i32 / 3;
    let r2 = 150i32 * 150;
    for y in 0..h {
        for x in 0..w {
            let gx = (x * 255 / (w - 1)) as i32;
            let gy = (y * 255 / (h - 1)) as i32;
            let mut r = gx;
            let mut g = gy;
            let mut b = 128;
            if x > w / 2 {
                g = 255 - g;
            }
            if (h / 3..h / 3 + 4).contains(&y) {
                r = 255;
                b = 0;
            }
            let dx = x as i32 - cx;
            let dy = y as i32 - cy;
            if dx * dx + dy * dy < r2 {
                r = 0;
                g = 0;
                b = 255;
            }
            let i = y * w + x;
            buf.data[i] = r.clamp(0, 255) as u8;
            buf.data[n + i] = g.clamp(0, 255) as u8;
            buf.data[2 * n + i] = b.clamp(0, 255) as u8;
        }
    }
    buf
}

fn kernel() -> [[f64; 5]; 5] {
    let mut k = [[0.0f64; 5]; 5];
    k[2][2] = 5.0;
    k[1][2] = -1.0;
    k[3][2] = -1.0;
    k[2][1] = -1.0;
    k[2][3] = -1.0;
    k
}

fn filters() -> Vec<(&'static str, Filter)> {
    vec![
        ("GaussianBlur", Filter::GaussianBlur { radius: 50.0 }),
        ("BoxBlur", Filter::BoxBlur { radius: 20 }),
        (
            "ShapeBlur",
            Filter::ShapeBlur {
                shape: 0,
                radius: 46,
            },
        ),
        (
            "SmartBlur",
            Filter::SmartBlur {
                radius: 50.0,
                threshold: 25.0,
                quality: Quality::Best,
                mode: SmartBlurMode::Normal,
            },
        ),
        (
            "MotionBlur",
            Filter::MotionBlur {
                angle: 30.0,
                distance: 60,
            },
        ),
        (
            "SurfaceBlur",
            Filter::SurfaceBlur {
                radius: 10,
                threshold: 20,
            },
        ),
        (
            "UnsharpMask",
            Filter::UnsharpMask {
                amount: 150.0,
                radius: 5.0,
                threshold: 0,
            },
        ),
        ("HighPass", Filter::HighPass { radius: 10.0 }),
        ("Blur", Filter::Blur),
        ("BlurMore", Filter::BlurMore),
        ("Sharpen", Filter::Sharpen),
        ("SharpenMore", Filter::SharpenMore),
        ("SharpenEdges", Filter::SharpenEdges),
        ("Median", Filter::Median { radius: 5 }),
        ("Despeckle", Filter::Despeckle),
        ("Maximum", Filter::Maximum { radius: 5 }),
        ("Minimum", Filter::Minimum { radius: 5 }),
        (
            "AddNoise",
            Filter::AddNoise {
                amount: 50.0,
                distribution: NoiseDistribution::Gaussian,
                monochromatic: false,
                seed: 1,
            },
        ),
        (
            "Emboss",
            Filter::Emboss {
                angle: 135.0,
                height: 3.0,
                amount: 100.0,
            },
        ),
        ("FindEdges", Filter::FindEdges),
        ("Solarize", Filter::Solarize),
        (
            "Custom",
            Filter::Custom {
                kernel: kernel(),
                scale: 1.0,
                offset: 0.0,
            },
        ),
        (
            "Offset",
            Filter::Offset {
                horizontal: 40,
                vertical: 40,
                wrap: true,
                background: [0, 0, 0],
            },
        ),
        ("Mosaic", Filter::Mosaic { cell_size: 20 }),
        (
            "Crystallize",
            Filter::Crystallize {
                cell_size: 20,
                seed: 1,
            },
        ),
        ("Facet", Filter::Facet),
        ("Fragment", Filter::Fragment),
        (
            "Mezzotint",
            Filter::Mezzotint {
                kind: MezzotintType::MediumDots,
                seed: 1,
            },
        ),
        (
            "Pointillize",
            Filter::Pointillize {
                cell_size: 20,
                background: [0, 0, 0],
                seed: 1,
            },
        ),
        (
            "ColorHalftone",
            Filter::ColorHalftone {
                max_radius: 10,
                angles: [108.0, 162.0, 90.0, 45.0],
            },
        ),
        ("Twirl", Filter::Twirl { angle: 90.0 }),
        ("Pinch", Filter::Pinch { amount: 50.0 }),
        (
            "Spherize",
            Filter::Spherize {
                amount: 100.0,
                mode: SpherizeMode::Normal,
            },
        ),
        (
            "Ripple",
            Filter::Ripple {
                amount: 100.0,
                size: RippleSize::Medium,
            },
        ),
        (
            "Wave",
            Filter::Wave {
                generators: 5,
                wavelength: (100.0, 300.0),
                amplitude: (5.0, 20.0),
                kind: WaveType::Sine,
                scale: (100.0, 100.0),
                seed: 1,
                repeat_edge: true,
            },
        ),
        (
            "PolarCoordinates",
            Filter::PolarCoordinates {
                kind: PolarKind::RectangularToPolar,
            },
        ),
        (
            "Shear",
            Filter::Shear {
                curve: vec![(-1.0, -0.5), (0.0, 0.0), (1.0, 0.5)],
                fill: ShearFill::WrapAround,
            },
        ),
        (
            "ZigZag",
            Filter::ZigZag {
                amount: 50.0,
                ridges: 5,
                style: ZigZagStyle::AroundCenter,
            },
        ),
        (
            "OceanRipple",
            Filter::OceanRipple {
                size: 9,
                magnitude: 5,
                seed: 1,
            },
        ),
        (
            "Clouds",
            Filter::Clouds {
                color_a: [0, 0, 0],
                color_b: [255, 255, 255],
                starker: false,
                seed: 1,
            },
        ),
        (
            "Fibers",
            Filter::Fibers {
                variance: 16.0,
                strength: 4.0,
                color_a: [0, 0, 0],
                color_b: [255, 255, 255],
                seed: 1,
            },
        ),
        (
            "LensFlare",
            Filter::LensFlare {
                brightness: 100.0,
                center: (0.5, 0.5),
                lens: LensType::Zoom,
            },
        ),
        (
            "DryBrush",
            Filter::DryBrush {
                brush_size: 8,
                brush_detail: 6,
                texture: 2,
                seed: 1,
            },
        ),
        (
            "PaintDaubs",
            Filter::PaintDaubs {
                brush_size: 8,
                sharpness: 20,
                brush_type: BrushType::Simple,
                seed: 1,
            },
        ),
        (
            "PaletteKnife",
            Filter::PaletteKnife {
                stroke_size: 12,
                stroke_detail: 2,
                softness: 8,
            },
        ),
        (
            "Watercolor",
            Filter::Watercolor {
                brush_detail: 8,
                shadow_intensity: 6,
                texture: 2,
                seed: 1,
            },
        ),
        (
            "ColoredPencil",
            Filter::ColoredPencil {
                pencil_width: 6,
                stroke_pressure: 8,
                paper_brightness: 10,
                background: [255, 255, 255],
                seed: 1,
            },
        ),
        (
            "Sponge",
            Filter::Sponge {
                brush_size: 6,
                definition: 18,
                smoothness: 4,
                seed: 1,
            },
        ),
        (
            "Crosshatch",
            Filter::Crosshatch {
                stroke_length: 50,
                sharpness: 20,
                strength: 3,
            },
        ),
        (
            "Spatter",
            Filter::Spatter {
                spray_radius: 25,
                smoothness: 15,
                seed: 1,
            },
        ),
        (
            "SumiE",
            Filter::SumiE {
                stroke_width: 15,
                stroke_pressure: 15,
                contrast: 40,
            },
        ),
        (
            "BasRelief",
            Filter::BasRelief {
                detail: 15,
                smoothness: 15,
                light_direction: LightDirection::Bottom,
                foreground: [0, 0, 0],
                background: [255, 255, 255],
            },
        ),
        (
            "ConteCrayon",
            Filter::ConteCrayon {
                foreground_level: 15,
                background_level: 15,
                texture: TextureOptions::default(),
                foreground: [0, 0, 0],
                background: [255, 255, 255],
                seed: 1,
            },
        ),
        (
            "StainedGlass",
            Filter::StainedGlass {
                cell_size: 50,
                border_thickness: 20,
                light_intensity: 10,
                foreground: [0, 0, 0],
                seed: 1,
            },
        ),
        (
            "OilPaint",
            Filter::OilPaint {
                stylization: 8.0,
                cleanliness: 5.0,
                scale: 8.0,
                bristle_detail: 5.0,
                angular_direction: 85.0,
                shine: 5.0,
            },
        ),
    ]
}

#[ignore = "1024x1024 all-kernel profile; run explicitly with --ignored --nocapture"]
#[test]
fn filter_profile_1024() {
    let base = structured_rgb(SIZE);
    let mut timings: Vec<(&'static str, f64)> = Vec::new();

    println!("\n=== filter profile @ {SIZE}x{SIZE} RGB (best of 2) ===");
    for (name, filter) in filters() {
        let mut best = f64::MAX;
        for _ in 0..2 {
            let mut buf = base.clone();
            let t = Instant::now();
            let res = apply(&filter, &mut buf);
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            if let Err(e) = res {
                eprintln!("{name}: {e}");
            }
            best = best.min(ms);
        }
        println!("{name:<18} {best:>9.1} ms");
        timings.push((name, best));
    }

    timings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    println!("\n=== summary (descending ms) ===");
    for (name, ms) in &timings {
        println!("{name:<18} {ms:>9.1} ms");
    }
}
