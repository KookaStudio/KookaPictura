use std::sync::Arc;

use crate::{
    AwaParams, BlurNoise, BlurPath, BlurPin, BokehParams, BrushType, ContourEdge, DepthMap,
    DiffuseMode, DisplaceFit, DisplaceMap, Durability, ExtrudeType, FrameOptions, GlassTexture,
    GrainType, HalftoneType, HdrToningParams, IrisPin, IrisShape, LensCorrectionParams, LensType,
    LightDirection, Lighting, LiquifyMesh, MezzotintType, NoiseDistribution, PathMotion, PolarKind,
    Quality, RadialMethod, ReduceNoiseParams, RippleSize, SharpenRemove, ShearFill, SmartBlurMode,
    SpherizeMode, SpinPin, SpinStrobe, StrokeDirection, TextureOptions, TileFill, TiltPin,
    TonalFade, VpEdit, WatermarkKind, WaveType, WindMethod, ZigZagStyle,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    GaussianBlur {
        radius: f64,
    },
    BoxBlur {
        radius: u32,
    },
    /// `shape` indexes `pictura_core::shape::CUSTOM_SHAPE_NAMES`.
    ShapeBlur {
        shape: usize,
        radius: u32,
    },
    MotionBlur {
        angle: f64,
        distance: u32,
    },
    RadialBlur {
        method: RadialMethod,
        amount: f64,
        /// Blur origin normalized to the image, `(0.5, 0.5)` the centre.
        center: (f64, f64),
        quality: Quality,
    },
    Average,
    Blur,
    BlurMore,
    SurfaceBlur {
        radius: u32,
        threshold: u8,
    },
    /// `quality` Draft / Good / Best are CS6's Low / Medium / High.
    SmartBlur {
        radius: f64,
        threshold: f64,
        quality: Quality,
        mode: SmartBlurMode,
    },
    LensBlur {
        depth: Option<Arc<DepthMap>>,
        focal: u8,
        invert: bool,
        shape: IrisShape,
        radius: u32,
        curvature: u8,
        rotation: f64,
        specular_brightness: u8,
        specular_threshold: u8,
        noise_amount: u8,
        noise_distribution: NoiseDistribution,
        monochromatic: bool,
        faster: bool,
        seed: u64,
    },
    /// Blur Gallery Field Blur: each pin carries a radius in buffer pixels and
    /// a position as a fraction of the buffer (outside allowed), with the
    /// gallery's Blur Effects (bokeh) and Noise post-passes.
    FieldBlur {
        pins: Vec<BlurPin>,
        bokeh: BokehParams,
        noise: BlurNoise,
    },
    /// Blur Gallery Iris Blur: elliptical sharp islands with a feathered fade
    /// to each pin's exterior radius (buffer pixels), with the same effects
    /// passes as Field Blur.
    IrisBlur {
        pins: Vec<IrisPin>,
        bokeh: BokehParams,
        noise: BlurNoise,
    },
    /// Blur Gallery Tilt-Shift: sharp bands fading to each pin's full radius
    /// (buffer pixels) on either side, with an optional distortion smear and
    /// the same effects passes as Field Blur.
    TiltShift {
        pins: Vec<TiltPin>,
        bokeh: BokehParams,
        noise: BlurNoise,
    },
    /// Blur Gallery Path Blur (post-CS6, non-parity): motion blur along the
    /// field the paths' tangents and end-point speeds (buffer pixels) blend
    /// into, with the Noise pass.
    PathBlur {
        paths: Vec<BlurPath>,
        motion: PathMotion,
        noise: BlurNoise,
    },
    /// Blur Gallery Spin Blur (post-CS6, non-parity): rotational blur along
    /// arcs round each spin's rotation centre inside its feathered ellipse,
    /// with the Noise pass.
    SpinBlur {
        spins: Vec<SpinPin>,
        strobe: SpinStrobe,
        noise: BlurNoise,
    },
    /// The Blur Gallery with several effects on: each list's radius field as
    /// its own kernel builds it, combined per pixel by the largest, then the
    /// path and spin gathers over the result; an empty list turns that effect
    /// off.
    BlurGallery {
        field: Vec<BlurPin>,
        iris: Vec<IrisPin>,
        tilt: Vec<TiltPin>,
        path: Vec<BlurPath>,
        motion: PathMotion,
        spin: Vec<SpinPin>,
        spin_strobe: SpinStrobe,
        bokeh: BokehParams,
        noise: BlurNoise,
    },
    Sharpen,
    SharpenMore,
    SharpenEdges,
    UnsharpMask {
        amount: f64,
        radius: f64,
        threshold: u8,
    },
    AddNoise {
        amount: f64,
        distribution: NoiseDistribution,
        monochromatic: bool,
        seed: u64,
    },
    Median {
        radius: u32,
    },
    Despeckle,
    /// Edge-preserving luminance/chroma denoise (`FILT-030`). A value kernel:
    /// alpha is left bit-identical.
    ReduceNoise(ReduceNoiseParams),
    Maximum {
        radius: u32,
    },
    Minimum {
        radius: u32,
    },
    Offset {
        horizontal: i32,
        vertical: i32,
        wrap: bool,
        /// Read past an edge from the nearest edge sample when true; takes
        /// precedence over `wrap`.
        repeat: bool,
        background: [u8; 3],
        /// CS6's "Set to Transparent" for the undefined area (vs "Set to
        /// Background", which paints `background` opaque).
        transparent: bool,
    },
    HighPass {
        radius: f64,
    },
    Custom {
        kernel: [[f64; 5]; 5],
        scale: f64,
        offset: f64,
    },
    Emboss {
        angle: f64,
        height: f64,
        amount: f64,
    },
    FindEdges,
    Solarize,
    NtscColors,
    Diffuse {
        mode: DiffuseMode,
    },
    GlowingEdges {
        width: u32,
        brightness: u32,
        smoothness: u32,
    },
    Mosaic {
        cell_size: u32,
    },
    Crystallize {
        cell_size: u32,
        seed: u64,
    },
    Facet,
    Fragment,
    Mezzotint {
        kind: MezzotintType,
        seed: u64,
    },
    Pointillize {
        cell_size: u32,
        background: [u8; 3],
        seed: u64,
    },
    ColorHalftone {
        max_radius: u32,
        angles: [f64; 4],
    },
    Twirl {
        angle: f64,
    },
    Pinch {
        amount: f64,
    },
    Spherize {
        amount: f64,
        mode: SpherizeMode,
    },
    Ripple {
        amount: f64,
        size: RippleSize,
    },
    Wave {
        generators: u32,
        wavelength: (f64, f64),
        amplitude: (f64, f64),
        kind: WaveType,
        scale: (f64, f64),
        seed: u64,
        repeat_edge: bool,
    },
    PolarCoordinates {
        kind: PolarKind,
    },
    Shear {
        curve: Vec<(f64, f64)>,
        fill: ShearFill,
    },
    /// Scales are percentages: 100 moves a pixel up to 128 px.
    Displace {
        map: Arc<DisplaceMap>,
        horizontal_scale: f64,
        vertical_scale: f64,
        fit: DisplaceFit,
        wrap: bool,
    },
    /// The Liquify dialog's mesh, stretched to fit a layer of another size.
    Liquify {
        mesh: Arc<LiquifyMesh>,
    },
    /// The Adaptive Wide Angle workspace's correction and constraints.
    AdaptiveWideAngle {
        params: AwaParams,
    },
    /// The Lens Correction dialog's profile and Custom settings.
    LensCorrection {
        params: LensCorrectionParams,
    },
    /// A Vanishing Point session's committed edit (its rendered grids).
    VanishingPoint {
        edit: Arc<VpEdit>,
    },
    ZigZag {
        amount: f64,
        ridges: u32,
        style: ZigZagStyle,
    },
    OceanRipple {
        size: u32,
        magnitude: u32,
        seed: u64,
    },
    Glass {
        distortion: u32,
        smoothness: u32,
        texture: GlassTexture,
        scaling: u32,
        invert: bool,
    },
    DiffuseGlow {
        graininess: u32,
        glow_amount: u32,
        clear_amount: u32,
        seed: u64,
    },
    Clouds {
        color_a: [u8; 3],
        color_b: [u8; 3],
        starker: bool,
        seed: u64,
    },
    DifferenceClouds {
        color_a: [u8; 3],
        color_b: [u8; 3],
        starker: bool,
        seed: u64,
    },
    Fibers {
        variance: f64,
        strength: f64,
        color_a: [u8; 3],
        color_b: [u8; 3],
        seed: u64,
    },
    PictureFrame {
        options: FrameOptions,
    },
    LensFlare {
        brightness: f64,
        center: (f64, f64),
        lens: LensType,
    },
    Lighting {
        lighting: Lighting,
    },
    Cutout {
        levels: u8,
        edge_simplicity: u8,
        edge_fidelity: u8,
    },
    FilmGrain {
        grain: u8,
        highlight_area: u8,
        intensity: u8,
        seed: u64,
    },
    NeonGlow {
        glow_size: i32,
        glow_brightness: u8,
        glow_color: [u8; 3],
    },
    PosterEdges {
        edge_thickness: u8,
        edge_intensity: u8,
        posterization: u8,
    },
    PaintDaubs {
        brush_size: u8,
        sharpness: u8,
        brush_type: BrushType,
        seed: u64,
    },
    PaletteKnife {
        stroke_size: u8,
        stroke_detail: u8,
        softness: u8,
    },
    PlasticWrap {
        highlight_strength: u8,
        detail: u8,
        smoothness: u8,
    },
    Sponge {
        brush_size: u8,
        definition: u8,
        smoothness: u8,
        seed: u64,
    },
    ColoredPencil {
        pencil_width: u8,
        stroke_pressure: u8,
        paper_brightness: u8,
        background: [u8; 3],
        seed: u64,
    },
    DryBrush {
        brush_size: u8,
        brush_detail: u8,
        texture: u8,
        seed: u64,
    },
    Fresco {
        brush_size: u8,
        brush_detail: u8,
        texture: u8,
        seed: u64,
    },
    RoughPastels {
        stroke_length: u8,
        stroke_detail: u8,
        texture: TextureOptions,
        seed: u64,
    },
    SmudgeStick {
        stroke_length: u8,
        highlight_area: u8,
        intensity: u8,
        seed: u64,
    },
    Underpainting {
        brush_size: u8,
        texture_coverage: u8,
        texture: TextureOptions,
        seed: u64,
    },
    Watercolor {
        brush_detail: u8,
        shadow_intensity: u8,
        texture: u8,
        seed: u64,
    },
    AccentedEdges {
        edge_width: u8,
        edge_brightness: u8,
        smoothness: u8,
    },
    AngledStrokes {
        direction_balance: u8,
        stroke_length: u8,
        sharpness: u8,
    },
    Crosshatch {
        stroke_length: u8,
        sharpness: u8,
        strength: u8,
    },
    DarkStrokes {
        balance: u8,
        black_intensity: u8,
        white_intensity: u8,
    },
    InkOutlines {
        stroke_length: u8,
        dark_intensity: u8,
        light_intensity: u8,
    },
    Spatter {
        spray_radius: u8,
        smoothness: u8,
        seed: u64,
    },
    SprayedStrokes {
        stroke_length: u8,
        spray_radius: u8,
        direction: StrokeDirection,
        seed: u64,
    },
    SumiE {
        stroke_width: u8,
        stroke_pressure: u8,
        contrast: u8,
    },
    BasRelief {
        detail: u8,
        smoothness: u8,
        light_direction: LightDirection,
        foreground: [u8; 3],
        background: [u8; 3],
    },
    ChalkCharcoal {
        charcoal_area: u8,
        chalk_area: u8,
        stroke_pressure: u8,
        foreground: [u8; 3],
        background: [u8; 3],
        seed: u64,
    },
    Charcoal {
        thickness: u8,
        detail: u8,
        light_dark_balance: u8,
        foreground: [u8; 3],
        background: [u8; 3],
        seed: u64,
    },
    Chrome {
        detail: u8,
        smoothness: u8,
    },
    ConteCrayon {
        foreground_level: u8,
        background_level: u8,
        texture: TextureOptions,
        foreground: [u8; 3],
        background: [u8; 3],
        seed: u64,
    },
    GraphicPen {
        stroke_length: u8,
        light_dark_balance: u8,
        direction: StrokeDirection,
        foreground: [u8; 3],
        background: [u8; 3],
    },
    HalftonePattern {
        size: u8,
        contrast: u8,
        pattern: HalftoneType,
    },
    NotePaper {
        image_balance: u8,
        graininess: u8,
        relief: u8,
        seed: u64,
    },
    Photocopy {
        detail: u8,
        darkness: u8,
    },
    Plaster {
        image_balance: u8,
        smoothness: u8,
        light_direction: LightDirection,
        foreground: [u8; 3],
        background: [u8; 3],
    },
    Reticulation {
        density: u8,
        black_level: u8,
        white_level: u8,
        foreground: [u8; 3],
        background: [u8; 3],
        seed: u64,
    },
    Stamp {
        light_dark_balance: u8,
        smoothness: u8,
        foreground: [u8; 3],
        background: [u8; 3],
    },
    TornEdges {
        image_balance: u8,
        smoothness: u8,
        contrast: u8,
        foreground: [u8; 3],
        background: [u8; 3],
    },
    WaterPaper {
        fiber_length: u8,
        brightness: u8,
        contrast: u8,
        seed: u64,
    },
    Craquelure {
        crack_spacing: u8,
        crack_depth: u8,
        crack_brightness: u8,
    },
    Grain {
        intensity: u8,
        contrast: u8,
        grain_type: GrainType,
        background: [u8; 3],
        seed: u64,
    },
    MosaicTiles {
        tile_size: u8,
        grout_width: u8,
        lighten_grout: u8,
        seed: u64,
    },
    Patchwork {
        square_size: u8,
        relief: u8,
        seed: u64,
    },
    StainedGlass {
        cell_size: u8,
        border_thickness: u8,
        light_intensity: u8,
        foreground: [u8; 3],
        seed: u64,
    },
    Texturizer {
        texture: TextureOptions,
    },
    OilPaint {
        stylization: f64,
        cleanliness: f64,
        scale: f64,
        bristle_detail: f64,
        angular_direction: f64,
        shine: f64,
    },
    DustAndScratches {
        radius: u32,
        threshold: u32,
    },
    Extrude {
        kind: ExtrudeType,
        size: u32,
        depth: f32,
        level_based: bool,
        solid_front: bool,
        mask_incomplete: bool,
    },
    Tiles {
        count: u32,
        offset: u32,
        fill: TileFill,
        foreground: [u8; 3],
        background: [u8; 3],
    },
    TraceContour {
        level: u8,
        edge: ContourEdge,
    },
    Wind {
        method: WindMethod,
        from_right: bool,
    },
    SmartSharpen {
        amount: f64,
        radius: f64,
        reduce_noise: f64,
        remove: SharpenRemove,
        angle: f64,
        more_accurate: bool,
        shadow: TonalFade,
        highlight: TonalFade,
    },
    HdrToning(HdrToningParams),
    /// The in-tree approximate watermark (`FILT-105`): our own deterministic
    /// spread-spectrum carrier, not Digimarc Barcode.
    EmbedWatermark {
        kind: WatermarkKind,
        creator_id: u32,
        id: u32,
        durability: Durability,
    },
}
