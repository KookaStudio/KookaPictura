//! One default, valid instance per `Filter` variant. The per-variant tests
//! iterate this list so a new variant cannot be added without an instance.

use std::sync::Arc;

use pictura_filters::*;

pub fn all() -> Vec<Filter> {
    let pin = BlurPin {
        x: 0.5,
        y: 0.5,
        blur: 6.0,
    };
    let iris = IrisPin {
        x: 0.5,
        y: 0.5,
        radius_x: 0.1,
        radius_y: 0.1,
        blur: 6.0,
        ..IrisPin::default()
    };
    let path = BlurPath {
        points: vec![(0.3, 0.5), (0.7, 0.5)],
        start_speed: 12.0,
        end_speed: 12.0,
    };
    let spin = SpinPin {
        radius_x: 0.45,
        radius_y: 0.45,
        feather: 1.0,
        blur_angle: 120.0,
        ..SpinPin::default()
    };
    let fg = [20, 30, 40];
    let bg = [230, 220, 210];
    let texture = TextureOptions::default();
    vec![
        Filter::GaussianBlur { radius: 2.0 },
        Filter::BoxBlur { radius: 2 },
        Filter::ShapeBlur {
            shape: 0,
            radius: 3,
        },
        Filter::MotionBlur {
            angle: 30.0,
            distance: 5,
        },
        Filter::RadialBlur {
            method: RadialMethod::Zoom,
            amount: 20.0,
            center: (0.5, 0.5),
            quality: Quality::Good,
        },
        Filter::Average,
        Filter::Blur,
        Filter::BlurMore,
        Filter::SurfaceBlur {
            radius: 3,
            threshold: 30,
        },
        Filter::SmartBlur {
            radius: 4.0,
            threshold: 50.0,
            quality: Quality::Good,
            mode: SmartBlurMode::Normal,
        },
        Filter::LensBlur {
            depth: None,
            focal: 128,
            invert: false,
            shape: IrisShape::Triangle,
            radius: 6,
            curvature: 50,
            rotation: 30.0,
            specular_brightness: 100,
            specular_threshold: 0,
            noise_amount: 100,
            noise_distribution: NoiseDistribution::Gaussian,
            monochromatic: false,
            faster: false,
            seed: 3,
        },
        Filter::FieldBlur {
            pins: vec![pin],
            bokeh: BokehParams::default(),
            noise: BlurNoise::default(),
        },
        Filter::IrisBlur {
            pins: vec![iris],
            bokeh: BokehParams::default(),
            noise: BlurNoise::default(),
        },
        Filter::TiltShift {
            pins: vec![TiltPin::default()],
            bokeh: BokehParams::default(),
            noise: BlurNoise::default(),
        },
        Filter::PathBlur {
            paths: vec![path.clone()],
            motion: PathMotion::default(),
            noise: BlurNoise::default(),
        },
        Filter::SpinBlur {
            spins: vec![spin],
            strobe: SpinStrobe::default(),
            noise: BlurNoise::default(),
        },
        Filter::BlurGallery {
            field: vec![pin],
            iris: vec![iris],
            tilt: vec![TiltPin::default()],
            path: vec![path],
            motion: PathMotion::default(),
            spin: vec![spin],
            spin_strobe: SpinStrobe::default(),
            bokeh: BokehParams::default(),
            noise: BlurNoise::default(),
        },
        Filter::Sharpen,
        Filter::SharpenMore,
        Filter::SharpenEdges,
        Filter::UnsharpMask {
            amount: 150.0,
            radius: 2.0,
            threshold: 0,
        },
        Filter::AddNoise {
            amount: 40.0,
            distribution: NoiseDistribution::Gaussian,
            monochromatic: false,
            seed: 7,
        },
        Filter::Median { radius: 2 },
        Filter::Despeckle,
        Filter::ReduceNoise(ReduceNoiseParams {
            reduce_color_noise: 50.0,
            sharpen_details: 40.0,
            remove_jpeg_artifacts: true,
            channels: [
                ChannelDetail {
                    strength: 7.0,
                    preserve_details: 20.0,
                },
                ChannelDetail {
                    strength: 5.0,
                    preserve_details: 40.0,
                },
                ChannelDetail {
                    strength: 9.0,
                    preserve_details: 0.0,
                },
            ],
        }),
        Filter::Maximum { radius: 2 },
        Filter::Minimum { radius: 2 },
        Filter::Offset {
            horizontal: 5,
            vertical: -3,
            wrap: false,
            repeat: false,
            background: [0; 3],
            transparent: false,
        },
        Filter::HighPass { radius: 3.0 },
        Filter::Custom {
            kernel: {
                let mut kernel = [[0.0; 5]; 5];
                kernel[2][2] = 1.0;
                kernel
            },
            scale: 1.0,
            offset: 0.0,
        },
        Filter::Emboss {
            angle: 135.0,
            height: 3.0,
            amount: 100.0,
        },
        Filter::FindEdges,
        Filter::Solarize,
        Filter::NtscColors,
        Filter::Diffuse {
            mode: DiffuseMode::Normal,
        },
        Filter::GlowingEdges {
            width: 2,
            brightness: 6,
            smoothness: 5,
        },
        Filter::Mosaic { cell_size: 4 },
        Filter::Crystallize {
            cell_size: 5,
            seed: 7,
        },
        Filter::Facet,
        Filter::Fragment,
        Filter::Mezzotint {
            kind: MezzotintType::MediumDots,
            seed: 7,
        },
        Filter::Pointillize {
            cell_size: 4,
            background: [255; 3],
            seed: 7,
        },
        Filter::ColorHalftone {
            max_radius: 6,
            angles: [108.0, 162.0, 90.0, 45.0],
        },
        Filter::Twirl { angle: 50.0 },
        Filter::Pinch { amount: 50.0 },
        Filter::Spherize {
            amount: 50.0,
            mode: SpherizeMode::Normal,
        },
        Filter::Ripple {
            amount: 100.0,
            size: RippleSize::Medium,
        },
        Filter::Wave {
            generators: 5,
            wavelength: (10.0, 120.0),
            amplitude: (5.0, 35.0),
            kind: WaveType::Sine,
            scale: (100.0, 100.0),
            seed: 7,
            repeat_edge: false,
        },
        Filter::PolarCoordinates {
            kind: PolarKind::RectangularToPolar,
        },
        Filter::Shear {
            curve: vec![(-1.0, -0.5), (0.0, 0.0), (1.0, 0.5)],
            fill: ShearFill::WrapAround,
        },
        Filter::Displace {
            map: Arc::new(DisplaceMap::neutral()),
            horizontal_scale: 10.0,
            vertical_scale: 10.0,
            fit: DisplaceFit::StretchToFit,
            wrap: false,
        },
        Filter::Liquify {
            mesh: Arc::new(LiquifyMesh::new(24, 24)),
        },
        Filter::AdaptiveWideAngle {
            params: AwaParams::default(),
        },
        Filter::LensCorrection {
            params: LensCorrectionParams {
                distortion: 30.0,
                edge: LensEdge::Extension,
                ..Default::default()
            },
        },
        Filter::VanishingPoint {
            edit: Arc::new(VpEdit::default()),
        },
        Filter::ZigZag {
            amount: 20.0,
            ridges: 5,
            style: ZigZagStyle::PondRipples,
        },
        Filter::OceanRipple {
            size: 9,
            magnitude: 5,
            seed: 7,
        },
        Filter::Glass {
            distortion: 5,
            smoothness: 3,
            texture: GlassTexture::Frosted,
            scaling: 100,
            invert: false,
        },
        Filter::DiffuseGlow {
            graininess: 5,
            glow_amount: 5,
            clear_amount: 5,
            seed: 7,
        },
        Filter::Clouds {
            color_a: [0; 3],
            color_b: [255; 3],
            starker: false,
            seed: 7,
        },
        Filter::DifferenceClouds {
            color_a: [0; 3],
            color_b: [255; 3],
            starker: false,
            seed: 7,
        },
        Filter::Fibers {
            variance: 16.0,
            strength: 4.0,
            color_a: [0; 3],
            color_b: [255; 3],
            seed: 7,
        },
        Filter::PictureFrame {
            options: FrameOptions::default(),
        },
        Filter::LensFlare {
            brightness: 100.0,
            center: (0.5, 0.5),
            lens: LensType::Zoom,
        },
        Filter::Lighting {
            lighting: Lighting::default(),
        },
        Filter::Cutout {
            levels: 4,
            edge_simplicity: 4,
            edge_fidelity: 2,
        },
        Filter::FilmGrain {
            grain: 4,
            highlight_area: 0,
            intensity: 10,
            seed: 7,
        },
        Filter::NeonGlow {
            glow_size: 5,
            glow_brightness: 15,
            glow_color: [0, 120, 255],
        },
        Filter::PosterEdges {
            edge_thickness: 2,
            edge_intensity: 1,
            posterization: 2,
        },
        Filter::PaintDaubs {
            brush_size: 4,
            sharpness: 7,
            brush_type: BrushType::Sparkle,
            seed: 7,
        },
        Filter::PaletteKnife {
            stroke_size: 6,
            stroke_detail: 3,
            softness: 0,
        },
        Filter::PlasticWrap {
            highlight_strength: 15,
            detail: 9,
            smoothness: 7,
        },
        Filter::Sponge {
            brush_size: 2,
            definition: 12,
            smoothness: 5,
            seed: 7,
        },
        Filter::ColoredPencil {
            pencil_width: 4,
            stroke_pressure: 8,
            paper_brightness: 25,
            background: bg,
            seed: 7,
        },
        Filter::DryBrush {
            brush_size: 2,
            brush_detail: 8,
            texture: 1,
            seed: 7,
        },
        Filter::Fresco {
            brush_size: 2,
            brush_detail: 8,
            texture: 1,
            seed: 7,
        },
        Filter::RoughPastels {
            stroke_length: 6,
            stroke_detail: 4,
            texture,
            seed: 7,
        },
        Filter::SmudgeStick {
            stroke_length: 2,
            highlight_area: 0,
            intensity: 10,
            seed: 7,
        },
        Filter::Underpainting {
            brush_size: 6,
            texture_coverage: 16,
            texture,
            seed: 7,
        },
        Filter::Watercolor {
            brush_detail: 9,
            shadow_intensity: 1,
            texture: 1,
            seed: 7,
        },
        Filter::AccentedEdges {
            edge_width: 2,
            edge_brightness: 38,
            smoothness: 5,
        },
        Filter::AngledStrokes {
            direction_balance: 50,
            stroke_length: 15,
            sharpness: 3,
        },
        Filter::Crosshatch {
            stroke_length: 9,
            sharpness: 6,
            strength: 1,
        },
        Filter::DarkStrokes {
            balance: 5,
            black_intensity: 6,
            white_intensity: 2,
        },
        Filter::InkOutlines {
            stroke_length: 4,
            dark_intensity: 20,
            light_intensity: 10,
        },
        Filter::Spatter {
            spray_radius: 10,
            smoothness: 5,
            seed: 7,
        },
        Filter::SprayedStrokes {
            stroke_length: 12,
            spray_radius: 7,
            direction: StrokeDirection::RightDiagonal,
            seed: 7,
        },
        Filter::SumiE {
            stroke_width: 10,
            stroke_pressure: 2,
            contrast: 16,
        },
        Filter::BasRelief {
            detail: 13,
            smoothness: 3,
            light_direction: LightDirection::Top,
            foreground: fg,
            background: bg,
        },
        Filter::ChalkCharcoal {
            charcoal_area: 6,
            chalk_area: 6,
            stroke_pressure: 1,
            foreground: fg,
            background: bg,
            seed: 7,
        },
        Filter::Charcoal {
            thickness: 1,
            detail: 5,
            light_dark_balance: 50,
            foreground: fg,
            background: bg,
            seed: 7,
        },
        Filter::Chrome {
            detail: 4,
            smoothness: 7,
        },
        Filter::ConteCrayon {
            foreground_level: 11,
            background_level: 7,
            texture,
            foreground: fg,
            background: bg,
            seed: 7,
        },
        Filter::GraphicPen {
            stroke_length: 15,
            light_dark_balance: 50,
            direction: StrokeDirection::RightDiagonal,
            foreground: fg,
            background: bg,
        },
        Filter::HalftonePattern {
            size: 1,
            contrast: 5,
            pattern: HalftoneType::Dot,
        },
        Filter::NotePaper {
            image_balance: 25,
            graininess: 10,
            relief: 11,
            seed: 7,
        },
        Filter::Photocopy {
            detail: 7,
            darkness: 8,
        },
        Filter::Plaster {
            image_balance: 20,
            smoothness: 2,
            light_direction: LightDirection::Top,
            foreground: fg,
            background: bg,
        },
        Filter::Reticulation {
            density: 12,
            black_level: 40,
            white_level: 5,
            foreground: fg,
            background: bg,
            seed: 7,
        },
        Filter::Stamp {
            light_dark_balance: 25,
            smoothness: 5,
            foreground: fg,
            background: bg,
        },
        Filter::TornEdges {
            image_balance: 25,
            smoothness: 11,
            contrast: 17,
            foreground: fg,
            background: bg,
        },
        Filter::WaterPaper {
            fiber_length: 15,
            brightness: 60,
            contrast: 80,
            seed: 7,
        },
        Filter::Craquelure {
            crack_spacing: 15,
            crack_depth: 6,
            crack_brightness: 9,
        },
        Filter::Grain {
            intensity: 40,
            contrast: 50,
            grain_type: GrainType::Regular,
            background: bg,
            seed: 7,
        },
        Filter::MosaicTiles {
            tile_size: 6,
            grout_width: 2,
            lighten_grout: 9,
            seed: 7,
        },
        Filter::Patchwork {
            square_size: 4,
            relief: 8,
            seed: 7,
        },
        Filter::StainedGlass {
            cell_size: 6,
            border_thickness: 2,
            light_intensity: 3,
            foreground: fg,
            seed: 7,
        },
        Filter::Texturizer { texture },
        Filter::OilPaint {
            stylization: 8.0,
            cleanliness: 5.0,
            scale: 8.0,
            bristle_detail: 5.0,
            angular_direction: 85.0,
            shine: 5.0,
        },
        Filter::DustAndScratches {
            radius: 2,
            threshold: 10,
        },
        Filter::Extrude {
            kind: ExtrudeType::Blocks,
            size: 6,
            depth: 10.0,
            level_based: false,
            solid_front: false,
            mask_incomplete: false,
        },
        Filter::Tiles {
            count: 4,
            offset: 10,
            fill: TileFill::BackgroundColor,
            foreground: fg,
            background: bg,
        },
        Filter::TraceContour {
            level: 128,
            edge: ContourEdge::Upper,
        },
        Filter::Wind {
            method: WindMethod::Wind,
            from_right: false,
        },
        Filter::SmartSharpen {
            amount: 150.0,
            radius: 2.0,
            reduce_noise: 10.0,
            remove: SharpenRemove::GaussianBlur,
            angle: 0.0,
            more_accurate: true,
            shadow: TonalFade::default(),
            highlight: TonalFade::default(),
        },
        Filter::HdrToning(HdrToningParams::default()),
        Filter::EmbedWatermark {
            kind: WatermarkKind::ImageId,
            creator_id: 42,
            id: 1234,
            durability: Durability::Medium,
        },
    ]
}
