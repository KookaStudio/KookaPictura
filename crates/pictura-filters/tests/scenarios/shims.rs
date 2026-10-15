//! The replaced filters' old function signatures, routed through `apply`,
//! so their spec-scenario tests run unchanged against the ported engine.

#![allow(dead_code, clippy::too_many_arguments)]

use pictura_core::PixelBuffer;
use pictura_filters::*;

pub fn average(buf: &mut PixelBuffer) -> Result<(), FilterError> {
    apply(&Filter::Average, buf)
}

pub fn add(
    buf: &mut PixelBuffer,
    amount: f64,
    distribution: NoiseDistribution,
    monochromatic: bool,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::AddNoise {
            amount,
            distribution,
            monochromatic,
            seed,
        },
        buf,
    )
}

pub fn emboss(
    buf: &mut PixelBuffer,
    angle: f64,
    height: f64,
    amount: f64,
) -> Result<(), FilterError> {
    apply(
        &Filter::Emboss {
            angle,
            height,
            amount,
        },
        buf,
    )
}

pub fn find_edges(buf: &mut PixelBuffer) -> Result<(), FilterError> {
    apply(&Filter::FindEdges, buf)
}

pub fn diffuse(buf: &mut PixelBuffer, mode: DiffuseMode) -> Result<(), FilterError> {
    apply(&Filter::Diffuse { mode }, buf)
}

pub fn glowing_edges(
    buf: &mut PixelBuffer,
    width: u32,
    brightness: u32,
    smoothness: u32,
) -> Result<(), FilterError> {
    apply(
        &Filter::GlowingEdges {
            width,
            brightness,
            smoothness,
        },
        buf,
    )
}

pub fn mosaic(buf: &mut PixelBuffer, cell_size: u32) -> Result<(), FilterError> {
    apply(&Filter::Mosaic { cell_size }, buf)
}

pub fn crystallize(buf: &mut PixelBuffer, cell_size: u32, seed: u64) -> Result<(), FilterError> {
    apply(&Filter::Crystallize { cell_size, seed }, buf)
}

pub fn facet(buf: &mut PixelBuffer) -> Result<(), FilterError> {
    apply(&Filter::Facet, buf)
}

pub fn fragment(buf: &mut PixelBuffer) -> Result<(), FilterError> {
    apply(&Filter::Fragment, buf)
}

pub fn mezzotint(buf: &mut PixelBuffer, kind: MezzotintType, seed: u64) -> Result<(), FilterError> {
    apply(&Filter::Mezzotint { kind, seed }, buf)
}

pub fn pointillize(
    buf: &mut PixelBuffer,
    cell_size: u32,
    background: [u8; 3],
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::Pointillize {
            cell_size,
            background,
            seed,
        },
        buf,
    )
}

pub fn twirl(buf: &mut PixelBuffer, angle: f64) -> Result<(), FilterError> {
    apply(&Filter::Twirl { angle }, buf)
}

pub fn pinch(buf: &mut PixelBuffer, amount: f64) -> Result<(), FilterError> {
    apply(&Filter::Pinch { amount }, buf)
}

pub fn spherize(buf: &mut PixelBuffer, amount: f64, mode: SpherizeMode) -> Result<(), FilterError> {
    apply(&Filter::Spherize { amount, mode }, buf)
}

pub fn ripple(buf: &mut PixelBuffer, amount: f64, size: RippleSize) -> Result<(), FilterError> {
    apply(&Filter::Ripple { amount, size }, buf)
}

pub fn wave(
    buf: &mut PixelBuffer,
    generators: u32,
    wavelength: (f64, f64),
    amplitude: (f64, f64),
    kind: WaveType,
    scale: (f64, f64),
    seed: u64,
    repeat_edge: bool,
) -> Result<(), FilterError> {
    apply(
        &Filter::Wave {
            generators,
            wavelength,
            amplitude,
            kind,
            scale,
            seed,
            repeat_edge,
        },
        buf,
    )
}

pub fn polar_coordinates(buf: &mut PixelBuffer, kind: PolarKind) -> Result<(), FilterError> {
    apply(&Filter::PolarCoordinates { kind }, buf)
}

pub fn zigzag(
    buf: &mut PixelBuffer,
    amount: f64,
    ridges: u32,
    style: ZigZagStyle,
) -> Result<(), FilterError> {
    apply(
        &Filter::ZigZag {
            amount,
            ridges,
            style,
        },
        buf,
    )
}

pub fn cutout(
    buf: &mut PixelBuffer,
    levels: u8,
    edge_simplicity: u8,
    edge_fidelity: u8,
) -> Result<(), FilterError> {
    apply(
        &Filter::Cutout {
            levels,
            edge_simplicity,
            edge_fidelity,
        },
        buf,
    )
}

pub fn film_grain(
    buf: &mut PixelBuffer,
    grain: u8,
    highlight_area: u8,
    intensity: u8,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::FilmGrain {
            grain,
            highlight_area,
            intensity,
            seed,
        },
        buf,
    )
}

pub fn neon_glow(
    buf: &mut PixelBuffer,
    glow_size: i32,
    glow_brightness: u8,
    glow_color: [u8; 3],
) -> Result<(), FilterError> {
    apply(
        &Filter::NeonGlow {
            glow_size,
            glow_brightness,
            glow_color,
        },
        buf,
    )
}

pub fn poster_edges(
    buf: &mut PixelBuffer,
    edge_thickness: u8,
    edge_intensity: u8,
    posterization: u8,
) -> Result<(), FilterError> {
    apply(
        &Filter::PosterEdges {
            edge_thickness,
            edge_intensity,
            posterization,
        },
        buf,
    )
}

pub fn paint_daubs(
    buf: &mut PixelBuffer,
    brush_size: u8,
    sharpness: u8,
    brush_type: BrushType,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::PaintDaubs {
            brush_size,
            sharpness,
            brush_type,
            seed,
        },
        buf,
    )
}

pub fn palette_knife(
    buf: &mut PixelBuffer,
    stroke_size: u8,
    stroke_detail: u8,
    softness: u8,
) -> Result<(), FilterError> {
    apply(
        &Filter::PaletteKnife {
            stroke_size,
            stroke_detail,
            softness,
        },
        buf,
    )
}

pub fn plastic_wrap(
    buf: &mut PixelBuffer,
    highlight_strength: u8,
    detail: u8,
    smoothness: u8,
) -> Result<(), FilterError> {
    apply(
        &Filter::PlasticWrap {
            highlight_strength,
            detail,
            smoothness,
        },
        buf,
    )
}

pub fn sponge(
    buf: &mut PixelBuffer,
    brush_size: u8,
    definition: u8,
    smoothness: u8,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::Sponge {
            brush_size,
            definition,
            smoothness,
            seed,
        },
        buf,
    )
}

pub fn colored_pencil(
    buf: &mut PixelBuffer,
    pencil_width: u8,
    stroke_pressure: u8,
    paper_brightness: u8,
    background: [u8; 3],
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::ColoredPencil {
            pencil_width,
            stroke_pressure,
            paper_brightness,
            background,
            seed,
        },
        buf,
    )
}

pub fn dry_brush(
    buf: &mut PixelBuffer,
    brush_size: u8,
    brush_detail: u8,
    texture: u8,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::DryBrush {
            brush_size,
            brush_detail,
            texture,
            seed,
        },
        buf,
    )
}

pub fn fresco(
    buf: &mut PixelBuffer,
    brush_size: u8,
    brush_detail: u8,
    texture: u8,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::Fresco {
            brush_size,
            brush_detail,
            texture,
            seed,
        },
        buf,
    )
}

pub fn rough_pastels(
    buf: &mut PixelBuffer,
    stroke_length: u8,
    stroke_detail: u8,
    texture: TextureOptions,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::RoughPastels {
            stroke_length,
            stroke_detail,
            texture,
            seed,
        },
        buf,
    )
}

pub fn smudge_stick(
    buf: &mut PixelBuffer,
    stroke_length: u8,
    highlight_area: u8,
    intensity: u8,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::SmudgeStick {
            stroke_length,
            highlight_area,
            intensity,
            seed,
        },
        buf,
    )
}

pub fn underpainting(
    buf: &mut PixelBuffer,
    brush_size: u8,
    texture_coverage: u8,
    texture: TextureOptions,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::Underpainting {
            brush_size,
            texture_coverage,
            texture,
            seed,
        },
        buf,
    )
}

pub fn watercolor(
    buf: &mut PixelBuffer,
    brush_detail: u8,
    shadow_intensity: u8,
    texture: u8,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::Watercolor {
            brush_detail,
            shadow_intensity,
            texture,
            seed,
        },
        buf,
    )
}

pub fn accented_edges(
    buf: &mut PixelBuffer,
    edge_width: u8,
    edge_brightness: u8,
    smoothness: u8,
) -> Result<(), FilterError> {
    apply(
        &Filter::AccentedEdges {
            edge_width,
            edge_brightness,
            smoothness,
        },
        buf,
    )
}

pub fn angled_strokes(
    buf: &mut PixelBuffer,
    direction_balance: u8,
    stroke_length: u8,
    sharpness: u8,
) -> Result<(), FilterError> {
    apply(
        &Filter::AngledStrokes {
            direction_balance,
            stroke_length,
            sharpness,
        },
        buf,
    )
}

pub fn crosshatch(
    buf: &mut PixelBuffer,
    stroke_length: u8,
    sharpness: u8,
    strength: u8,
) -> Result<(), FilterError> {
    apply(
        &Filter::Crosshatch {
            stroke_length,
            sharpness,
            strength,
        },
        buf,
    )
}

pub fn dark_strokes(
    buf: &mut PixelBuffer,
    balance: u8,
    black_intensity: u8,
    white_intensity: u8,
) -> Result<(), FilterError> {
    apply(
        &Filter::DarkStrokes {
            balance,
            black_intensity,
            white_intensity,
        },
        buf,
    )
}

pub fn ink_outlines(
    buf: &mut PixelBuffer,
    stroke_length: u8,
    dark_intensity: u8,
    light_intensity: u8,
) -> Result<(), FilterError> {
    apply(
        &Filter::InkOutlines {
            stroke_length,
            dark_intensity,
            light_intensity,
        },
        buf,
    )
}

pub fn spatter(
    buf: &mut PixelBuffer,
    spray_radius: u8,
    smoothness: u8,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::Spatter {
            spray_radius,
            smoothness,
            seed,
        },
        buf,
    )
}

pub fn sprayed_strokes(
    buf: &mut PixelBuffer,
    stroke_length: u8,
    spray_radius: u8,
    direction: StrokeDirection,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::SprayedStrokes {
            stroke_length,
            spray_radius,
            direction,
            seed,
        },
        buf,
    )
}

pub fn sumi_e(
    buf: &mut PixelBuffer,
    stroke_width: u8,
    stroke_pressure: u8,
    contrast: u8,
) -> Result<(), FilterError> {
    apply(
        &Filter::SumiE {
            stroke_width,
            stroke_pressure,
            contrast,
        },
        buf,
    )
}

pub fn bas_relief(
    buf: &mut PixelBuffer,
    detail: u8,
    smoothness: u8,
    light_direction: LightDirection,
    foreground: [u8; 3],
    background: [u8; 3],
) -> Result<(), FilterError> {
    apply(
        &Filter::BasRelief {
            detail,
            smoothness,
            light_direction,
            foreground,
            background,
        },
        buf,
    )
}

pub fn chalk_charcoal(
    buf: &mut PixelBuffer,
    charcoal_area: u8,
    chalk_area: u8,
    stroke_pressure: u8,
    foreground: [u8; 3],
    background: [u8; 3],
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::ChalkCharcoal {
            charcoal_area,
            chalk_area,
            stroke_pressure,
            foreground,
            background,
            seed,
        },
        buf,
    )
}

pub fn charcoal(
    buf: &mut PixelBuffer,
    thickness: u8,
    detail: u8,
    light_dark_balance: u8,
    foreground: [u8; 3],
    background: [u8; 3],
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::Charcoal {
            thickness,
            detail,
            light_dark_balance,
            foreground,
            background,
            seed,
        },
        buf,
    )
}

pub fn chrome(buf: &mut PixelBuffer, detail: u8, smoothness: u8) -> Result<(), FilterError> {
    apply(&Filter::Chrome { detail, smoothness }, buf)
}

pub fn conte_crayon(
    buf: &mut PixelBuffer,
    foreground_level: u8,
    background_level: u8,
    texture: TextureOptions,
    foreground: [u8; 3],
    background: [u8; 3],
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::ConteCrayon {
            foreground_level,
            background_level,
            texture,
            foreground,
            background,
            seed,
        },
        buf,
    )
}

pub fn graphic_pen(
    buf: &mut PixelBuffer,
    stroke_length: u8,
    light_dark_balance: u8,
    direction: StrokeDirection,
    foreground: [u8; 3],
    background: [u8; 3],
) -> Result<(), FilterError> {
    apply(
        &Filter::GraphicPen {
            stroke_length,
            light_dark_balance,
            direction,
            foreground,
            background,
        },
        buf,
    )
}

pub fn halftone_pattern(
    buf: &mut PixelBuffer,
    size: u8,
    contrast: u8,
    pattern: HalftoneType,
) -> Result<(), FilterError> {
    apply(
        &Filter::HalftonePattern {
            size,
            contrast,
            pattern,
        },
        buf,
    )
}

pub fn note_paper(
    buf: &mut PixelBuffer,
    image_balance: u8,
    graininess: u8,
    relief: u8,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::NotePaper {
            image_balance,
            graininess,
            relief,
            seed,
        },
        buf,
    )
}

pub fn photocopy(buf: &mut PixelBuffer, detail: u8, darkness: u8) -> Result<(), FilterError> {
    apply(&Filter::Photocopy { detail, darkness }, buf)
}

pub fn plaster(
    buf: &mut PixelBuffer,
    image_balance: u8,
    smoothness: u8,
    light_direction: LightDirection,
    foreground: [u8; 3],
    background: [u8; 3],
) -> Result<(), FilterError> {
    apply(
        &Filter::Plaster {
            image_balance,
            smoothness,
            light_direction,
            foreground,
            background,
        },
        buf,
    )
}

pub fn reticulation(
    buf: &mut PixelBuffer,
    density: u8,
    black_level: u8,
    white_level: u8,
    foreground: [u8; 3],
    background: [u8; 3],
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::Reticulation {
            density,
            black_level,
            white_level,
            foreground,
            background,
            seed,
        },
        buf,
    )
}

pub fn stamp(
    buf: &mut PixelBuffer,
    light_dark_balance: u8,
    smoothness: u8,
    foreground: [u8; 3],
    background: [u8; 3],
) -> Result<(), FilterError> {
    apply(
        &Filter::Stamp {
            light_dark_balance,
            smoothness,
            foreground,
            background,
        },
        buf,
    )
}

pub fn torn_edges(
    buf: &mut PixelBuffer,
    image_balance: u8,
    smoothness: u8,
    contrast: u8,
    foreground: [u8; 3],
    background: [u8; 3],
) -> Result<(), FilterError> {
    apply(
        &Filter::TornEdges {
            image_balance,
            smoothness,
            contrast,
            foreground,
            background,
        },
        buf,
    )
}

pub fn water_paper(
    buf: &mut PixelBuffer,
    fiber_length: u8,
    brightness: u8,
    contrast: u8,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::WaterPaper {
            fiber_length,
            brightness,
            contrast,
            seed,
        },
        buf,
    )
}

pub fn craquelure(
    buf: &mut PixelBuffer,
    crack_spacing: u8,
    crack_depth: u8,
    crack_brightness: u8,
) -> Result<(), FilterError> {
    apply(
        &Filter::Craquelure {
            crack_spacing,
            crack_depth,
            crack_brightness,
        },
        buf,
    )
}

pub fn grain(
    buf: &mut PixelBuffer,
    intensity: u8,
    contrast: u8,
    grain_type: GrainType,
    background: [u8; 3],
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::Grain {
            intensity,
            contrast,
            grain_type,
            background,
            seed,
        },
        buf,
    )
}

pub fn mosaic_tiles(
    buf: &mut PixelBuffer,
    tile_size: u8,
    grout_width: u8,
    lighten_grout: u8,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::MosaicTiles {
            tile_size,
            grout_width,
            lighten_grout,
            seed,
        },
        buf,
    )
}

pub fn patchwork(
    buf: &mut PixelBuffer,
    square_size: u8,
    relief: u8,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::Patchwork {
            square_size,
            relief,
            seed,
        },
        buf,
    )
}

pub fn stained_glass(
    buf: &mut PixelBuffer,
    cell_size: u8,
    border_thickness: u8,
    light_intensity: u8,
    foreground: [u8; 3],
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::StainedGlass {
            cell_size,
            border_thickness,
            light_intensity,
            foreground,
            seed,
        },
        buf,
    )
}

pub fn texturizer(buf: &mut PixelBuffer, texture: TextureOptions) -> Result<(), FilterError> {
    apply(&Filter::Texturizer { texture }, buf)
}

pub fn extrude(
    buf: &mut PixelBuffer,
    kind: ExtrudeType,
    size: u32,
    depth: f32,
    level_based: bool,
    solid_front: bool,
    mask_incomplete: bool,
) -> Result<(), FilterError> {
    apply(
        &Filter::Extrude {
            kind,
            size,
            depth,
            level_based,
            solid_front,
            mask_incomplete,
        },
        buf,
    )
}

pub fn tiles(
    buf: &mut PixelBuffer,
    count: u32,
    offset: u32,
    fill: TileFill,
    foreground: [u8; 3],
    background: [u8; 3],
) -> Result<(), FilterError> {
    apply(
        &Filter::Tiles {
            count,
            offset,
            fill,
            foreground,
            background,
        },
        buf,
    )
}

pub fn trace_contour(
    buf: &mut PixelBuffer,
    level: u8,
    edge: ContourEdge,
) -> Result<(), FilterError> {
    apply(&Filter::TraceContour { level, edge }, buf)
}

pub fn wind(
    buf: &mut PixelBuffer,
    method: WindMethod,
    from_right: bool,
) -> Result<(), FilterError> {
    apply(&Filter::Wind { method, from_right }, buf)
}

pub fn color_halftone(
    buf: &mut PixelBuffer,
    max_radius: u32,
    angles: [f64; 4],
) -> Result<(), FilterError> {
    apply(&Filter::ColorHalftone { max_radius, angles }, buf)
}

pub fn dust_and_scratches(
    buf: &mut PixelBuffer,
    radius: u32,
    threshold: u32,
) -> Result<(), FilterError> {
    apply(&Filter::DustAndScratches { radius, threshold }, buf)
}

pub fn solarize(buf: &mut PixelBuffer) -> Result<(), FilterError> {
    apply(&Filter::Solarize, buf)
}

pub fn shear(
    buf: &mut PixelBuffer,
    curve: &[(f64, f64)],
    fill: ShearFill,
) -> Result<(), FilterError> {
    apply(
        &Filter::Shear {
            curve: curve.to_vec(),
            fill,
        },
        buf,
    )
}

pub fn ocean_ripple(
    buf: &mut PixelBuffer,
    size: u32,
    magnitude: u32,
    seed: u64,
) -> Result<(), FilterError> {
    apply(
        &Filter::OceanRipple {
            size,
            magnitude,
            seed,
        },
        buf,
    )
}

pub fn median(buf: &mut PixelBuffer, radius: u32) -> Result<(), FilterError> {
    apply(&Filter::Median { radius }, buf)
}

pub fn despeckle(buf: &mut PixelBuffer) -> Result<(), FilterError> {
    apply(&Filter::Despeckle, buf)
}

pub fn reduce_noise(buf: &mut PixelBuffer, params: &ReduceNoiseParams) -> Result<(), FilterError> {
    apply(&Filter::ReduceNoise(*params), buf)
}

/// Rec. 601 luma, as the replaced tests measured brightness.
pub fn luma(r: f64, g: f64, b: f64) -> f64 {
    0.299 * r + 0.587 * g + 0.114 * b
}
