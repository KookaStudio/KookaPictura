use std::collections::BTreeSet;

use super::*;
use crate::filter::Filter;
use crate::{apply, FilterEngine, FilterKind, SmartBlurMode};
use pictura_core::PixelBuffer;

#[test]
fn catalogue_covers_every_kind() {
    let mut ids = BTreeSet::new();
    for &kind in FilterKind::ALL {
        let spec = kind.spec();
        assert_eq!(spec.kind, kind, "{kind:?} has the wrong catalogue row");
        assert!(ids.insert(spec.id), "duplicate catalogue id {}", spec.id);
    }
    assert_eq!(ids.len(), FilterKind::ALL.len());
}

#[test]
fn catalogue_rows_match_kind_order() {
    use FilterKind::*;
    #[rustfmt::skip]
    const EXPECTED: &[FilterKind] = &[
        Liquify, AdaptiveWideAngle, LensCorrection, VanishingPoint, OilPaint, ColoredPencil, Cutout, DryBrush, FilmGrain,
        Fresco, NeonGlow, PaintDaubs, PaletteKnife, PlasticWrap, PosterEdges, RoughPastels, SmudgeStick,
        Sponge, Underpainting, Watercolor, FieldBlur, IrisBlur, TiltShift, PathBlur, SpinBlur,
        GaussianBlur, MotionBlur, RadialBlur, BoxBlur, ShapeBlur, SurfaceBlur, SmartBlur, LensBlur,
        Average, Blur, BlurMore, BlurGallery, AccentedEdges, AngledStrokes, Crosshatch, DarkStrokes,
        InkOutlines, Spatter, SprayedStrokes, SumiE, Displace, DiffuseGlow, Glass, OceanRipple,
        Pinch, PolarCoordinates, Ripple, Shear, Spherize, Twirl, Wave, ZigZag,
        AddNoise, Despeckle, DustAndScratches, Median, ReduceNoise, ColorHalftone, Crystallize, Facet,
        Fragment, Mezzotint, Mosaic, Pointillize, Clouds, DifferenceClouds, Fibers, LensFlare,
        Lighting, PictureFrame, SmartSharpen, UnsharpMask, Sharpen, SharpenEdges, SharpenMore, BasRelief,
        ChalkCharcoal, Charcoal, Chrome, ConteCrayon, GraphicPen, HalftonePattern, NotePaper, Photocopy,
        Plaster, Reticulation, Stamp, TornEdges, WaterPaper, Emboss, Extrude, FindEdges,
        Solarize, Diffuse, GlowingEdges, Tiles, TraceContour, Wind, Craquelure, Grain,
        MosaicTiles, Patchwork, StainedGlass, Texturizer, Custom, HighPass, Maximum, Minimum,
        Offset, NtscColors, DeInterlace, HdrToning, EmbedWatermark,
    ];
    assert_eq!(FilterKind::ALL, EXPECTED);
    for (kind, row) in FilterKind::ALL.iter().zip(all()) {
        assert_eq!(row.kind, *kind, "{kind:?}");
    }
}

#[test]
fn catalogue_rows_round_trip_their_facts() {
    let gaussian = Filter::GaussianBlur { radius: 16.0 };
    assert!(gaussian.moves_alpha());
    assert_eq!(gaussian.preview_apron(), 16);
    assert_eq!(gaussian.engine(), FilterEngine::Native);

    let solarize = Filter::Solarize;
    assert!(!solarize.moves_alpha());
    assert_eq!(solarize.preview_apron(), 128);
    assert_eq!(solarize.engine(), FilterEngine::Native);

    let smart_normal = Filter::SmartBlur {
        radius: 3.0,
        threshold: 25.0,
        quality: crate::Quality::Good,
        mode: SmartBlurMode::Normal,
    };
    let smart_edges = Filter::SmartBlur {
        radius: 3.0,
        threshold: 25.0,
        quality: crate::Quality::Good,
        mode: SmartBlurMode::EdgeOnly,
    };
    assert!(smart_normal.moves_alpha());
    assert!(!smart_edges.moves_alpha());

    for &kind in FilterKind::ALL {
        let spec = kind.spec();
        assert_eq!(spec.engine, kind.engine());
        assert_eq!(spec.alpha.is_moving_kind(), kind.moves_alpha());
        assert_eq!(spec.gpu.is_cpu_preferred(), kind.cpu_outruns_gpu());
    }
}

#[test]
fn catalogue_defaults_are_in_range() {
    for &kind in FilterKind::ALL {
        let spec = kind.spec();
        assert!(
            spec.defaults_are_in_range(),
            "{} has a default outside its range",
            spec.id
        );
    }
}

#[test]
fn every_catalogue_row_builds_from_its_defaults() {
    for &kind in FilterKind::ALL {
        let spec = kind.spec();
        let defaults = spec.defaults();
        let built = build(spec.id, &[], &NullResources);
        assert!(built.is_some(), "{} does not build from defaults", spec.id);
        assert_eq!(
            built,
            build(spec.id, &defaults, &NullResources),
            "{} differs between an empty list and its defaults",
            spec.id
        );
    }
}

#[test]
fn hdr_toning_is_engine_only() {
    let spec = descriptor("hdr-toning").expect("HDR Toning row");
    assert_eq!(spec.kind, FilterKind::HdrToning);
    assert!(!spec.menu);
    assert!(spec.path.is_empty());
    assert!(spec.params.is_empty());
    assert_eq!(spec.gallery, None);
    assert!(FilterKind::ALL.contains(&FilterKind::HdrToning));
    assert!(default_filter(FilterKind::HdrToning).is_some());
}

#[test]
fn add_noise_amount_has_one_range() {
    let spec = descriptor("add-noise").expect("Add Noise row");
    assert_eq!(spec.range_for_slot(0), Some(0.1..=400.0));

    let mut buffer = PixelBuffer::new(4, 4, 4);
    buffer.data.fill(128);
    let accepted = Filter::AddNoise {
        amount: 0.1,
        distribution: crate::NoiseDistribution::Uniform,
        monochromatic: false,
        seed: 1,
    };
    apply(&accepted, &mut buffer.clone()).expect("0.1 is in range");

    let refused = Filter::AddNoise {
        amount: 0.0,
        distribution: crate::NoiseDistribution::Uniform,
        monochromatic: false,
        seed: 1,
    };
    assert!(apply(&refused, &mut buffer).is_err());
}

#[test]
fn surface_blur_threshold_has_one_range() {
    let spec = descriptor("surface-blur").expect("Surface Blur row");
    assert_eq!(spec.range_for_slot(1), Some(1.0..=255.0));

    let accepted = Filter::SurfaceBlur {
        radius: 1,
        threshold: 1,
    };
    apply(&accepted, &mut PixelBuffer::new(4, 4, 4)).expect("1 is in range");

    let refused = Filter::SurfaceBlur {
        radius: 1,
        threshold: 0,
    };
    assert!(apply(&refused, &mut PixelBuffer::new(4, 4, 4)).is_err());
}

#[test]
fn out_of_range_slot_is_refused() {
    let mut buffer = PixelBuffer::new(4, 4, 4);
    buffer.data.fill(128);
    let filter = Filter::AddNoise {
        amount: 400.1,
        distribution: crate::NoiseDistribution::Uniform,
        monochromatic: false,
        seed: 1,
    };
    assert!(apply(&filter, &mut buffer).is_err());

    let mut smart = descriptor("smart-blur").expect("Smart Blur row").defaults();
    smart[0] = 0.0;
    let smart = build("smart-blur", &smart, &NullResources).expect("Smart Blur builds");
    assert!(apply(&smart, &mut PixelBuffer::new(4, 4, 4)).is_err());

    let mut awa = descriptor("adaptive-wide-angle")
        .expect("Adaptive Wide Angle row")
        .defaults();
    awa[1] = 49.0;
    assert!(build("adaptive-wide-angle", &awa, &NullResources).is_none());
}

#[test]
fn engine_defaults_agree_with_the_catalogue() {
    assert_eq!(
        build("lighting-effects", &[], &NullResources),
        Some(Filter::Lighting {
            lighting: crate::Lighting::default(),
        })
    );
    assert_eq!(
        build("picture-frame", &[], &NullResources),
        Some(Filter::PictureFrame {
            options: crate::FrameOptions::default(),
        })
    );
    assert_eq!(
        build("smart-sharpen", &[], &NullResources),
        Some(Filter::SmartSharpen {
            amount: 100.0,
            radius: 1.0,
            reduce_noise: 0.0,
            remove: crate::SharpenRemove::GaussianBlur,
            angle: 0.0,
            more_accurate: false,
            shadow: crate::TonalFade::default(),
            highlight: crate::TonalFade::default(),
        })
    );
}

/// Records whether a build asked for any session resource.
#[derive(Default)]
struct Recording(std::cell::Cell<bool>);

impl FilterResources for Recording {
    fn depth_map(&self, _id: u32) -> Option<Arc<crate::DepthMap>> {
        self.0.set(true);
        None
    }

    fn displace_map(&self, id: u32) -> Option<Arc<crate::DisplaceMap>> {
        self.0.set(true);
        NullResources.displace_map(id)
    }

    fn liquify_mesh(&self, id: u32) -> Option<Arc<crate::LiquifyMesh>> {
        self.0.set(true);
        NullResources.liquify_mesh(id)
    }

    fn vanishing_point(&self, id: u32) -> Option<Arc<crate::VpEdit>> {
        self.0.set(true);
        NullResources.vanishing_point(id)
    }

    fn awa_constraints(&self, id: u32) -> Option<Arc<Vec<crate::AwaConstraint>>> {
        self.0.set(true);
        NullResources.awa_constraints(id)
    }
}

#[test]
fn resource_kinds_are_not_smart_capable() {
    for spec in all() {
        let defaults = spec.defaults();
        // Defaults, then each slot nudged through the first choice indices, so
        // a resource behind a non-default choice (Lens Blur's depth source) is
        // reached too.
        let mut variants = vec![defaults.clone()];
        for slot in 0..defaults.len() {
            for value in [1.0, 2.0, 3.0] {
                let mut params = defaults.clone();
                params[slot] = value;
                variants.push(params);
            }
        }
        let touched = variants.iter().any(|params| {
            let recording = Recording::default();
            let _ = build(spec.id, params, &recording);
            recording.0.get()
        });
        if touched {
            assert!(
                !smart_capable(spec.id),
                "{} reads a session resource but is smart-capable",
                spec.id
            );
        }
    }
}

#[test]
fn smart_capable_excludes_cs6_list() {
    for id in [
        "liquify",
        "vanishing-point",
        "lens-blur",
        "displace",
        "adaptive-wide-angle",
    ] {
        assert!(descriptor(id).is_some(), "{id} is a catalogue kind");
        assert!(!smart_capable(id), "{id} is excluded");
    }
    for id in ["gaussian-blur", "add-noise", "lens-correction", "emboss"] {
        assert!(smart_capable(id), "{id} is smart-capable");
    }
    assert!(!smart_capable("not-a-filter"));
}
