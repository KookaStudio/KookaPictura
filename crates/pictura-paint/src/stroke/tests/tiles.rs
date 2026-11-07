use super::*;
use crate::focus::FocusOptions;
use crate::smudge::SmudgeOptions;
use crate::tone::ToneOptions;

#[test]
fn a_stroke_forks_only_touched_bands() {
    let (w, h) = (8usize, 128usize);
    let mut layer = Layer {
        name: "px".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        },
        channels: vec![Channel {
            id: 0,
            data: vec![1u8; w * h].into(),
        }],
        ..Default::default()
    };
    layer.channels[0].data.fracture(w);
    let holder = layer.channels[0].data.clone();

    let ch: PlaneIndex = [Some(0), None, None, None];
    let mut scratch = vec![0u8; w * h];
    let mut applied = vec![0u8; w * h];
    let cuts = crate::stroke::raster::plane_slices(&mut layer, ch, w, h, 0, 64);
    let bands = crate::stroke::raster::split_bands(0, 64, w, &mut scratch, &mut applied, cuts);
    for mut band in bands {
        if let Some(plane) = band.planes[0].as_mut() {
            plane[0] = 9;
        }
    }

    assert!(
        !layer.channels[0].data.shares_band(0, &holder),
        "the touched band forked"
    );
    assert!(
        layer.channels[0].data.shares_band(1, &holder),
        "the untouched band stayed shared"
    );
    assert_eq!(holder.band(1).expect("band 1")[0], 1);
}

// [tsr_first_dab]
#[test]
fn a_tone_stroke_forks_only_touched_bands() {
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 256,
        right: 64,
    };
    let mut doc = offset_layer_doc(64, 256, rect, (120, 90, 60, 255));
    for channel in &mut doc.layers[0].channels {
        channel.data.fracture(64);
    }
    let history = doc.clone();
    let cfg = StrokeConfig {
        diameter: 20,
        ..StrokeConfig::default()
    };
    let mut stroke =
        Stroke::begin_tone(&doc, "0", cfg, ToneOptions::default()).expect("begin tone");
    assert!(stroke.sample(&mut doc, sample(20.0, 20.0)), "nothing toned");
    for (live, held) in doc.layers[0]
        .channels
        .iter()
        .zip(&history.layers[0].channels)
    {
        assert!(live.data.band_count() > 1, "the plane was flattened");
        assert!(
            live.data.shares_band(3, &held.data),
            "an untouched band was copied"
        );
    }
    let red = &doc.layers[0].channels[0].data;
    assert!(!red.shares_band(0, &history.layers[0].channels[0].data));
}

// [tsr_write_back]
#[test]
fn per_dab_write_back_agrees_banded_and_serial() {
    let original = textured_doc();
    let cfg = StrokeConfig {
        diameter: 60,
        hardness: 0,
        ..StrokeConfig::default()
    };
    type Begin = fn(&Document, StrokeConfig) -> Stroke;
    let cases: [(&str, Begin); 3] = [
        ("dodge", |doc, cfg| {
            Stroke::begin_tone(doc, "0", cfg, ToneOptions::default()).expect("begin")
        }),
        ("blur", |doc, cfg| {
            Stroke::begin_focus(doc, "0", cfg, FocusOptions::default(), None).expect("begin")
        }),
        ("smudge", |doc, cfg| {
            Stroke::begin_smudge(doc, "0", cfg, SmudgeOptions::default(), None).expect("begin")
        }),
    ];
    // Far apart, so one sample's dabs leave most of their union unpainted.
    let samples = [
        sample(30.0, 40.0),
        sample(140.0, 130.0),
        sample(20.0, 120.0),
    ];
    for (name, begin) in cases {
        let mut banded_doc = original.clone();
        let mut banded = begin(&banded_doc, cfg);
        let mut serial_doc = original.clone();
        let mut serial = begin(&serial_doc, cfg).serialized();
        for &s in &samples {
            assert_eq!(
                banded.sample(&mut banded_doc, s),
                serial.sample(&mut serial_doc, s),
                "{name}: changed differs"
            );
            assert_eq!(
                banded.take_dirty(),
                serial.take_dirty(),
                "{name}: dab dirty differs"
            );
        }
        assert!(banded_doc == serial_doc, "{name}: banded pixels differ");
        assert!(banded_doc != original, "{name}: the stroke painted nothing");
        banded.cancel(&mut banded_doc);
        assert!(banded_doc == original, "{name}: cancel left pixels behind");
    }
}

// [tsr_start]
#[test]
fn a_tone_stroke_loads_only_the_tiles_it_reaches() {
    let original = textured_doc();
    let cfg = StrokeConfig {
        diameter: 30,
        hardness: 0,
        ..StrokeConfig::default()
    };
    let samples = [sample(10.0, 20.0), sample(90.0, 75.0), sample(140.0, 140.0)];

    let mut lazy_doc = original.clone();
    let mut lazy = Stroke::begin_tone(&lazy_doc, "0", cfg, ToneOptions::default()).expect("begin");
    fn image(stroke: &Stroke) -> &RgbaImage {
        &stroke.per_dab.as_ref().expect("per dab").pixels
    }
    assert!(image(&lazy).data.iter().all(|p| *p == [0; 4]));

    let mut eager_doc = original.clone();
    let mut eager =
        Stroke::begin_tone(&eager_doc, "0", cfg, ToneOptions::default()).expect("begin");
    let target = layer_at(&eager_doc, &eager.layer_path).expect("layer");
    let per = eager.per_dab.as_mut().expect("per dab");
    per.pixels = layer_rgba(target);
    per.lazy = None;

    for &s in &samples {
        assert_eq!(
            lazy.sample(&mut lazy_doc, s),
            eager.sample(&mut eager_doc, s),
            "changed differs"
        );
        assert_eq!(lazy.take_dirty(), eager.take_dirty(), "dirty differs");
    }
    assert!(lazy_doc == eager_doc, "lazy pixels differ");
    assert!(lazy_doc != original, "the stroke painted nothing");
    assert!(
        image(&lazy).data.contains(&[0; 4]),
        "an unreached tile was loaded"
    );
}

#[test]
fn a_tone_stroke_refuses_a_layer_that_changed_under_it() {
    let original = textured_doc();
    let cfg = StrokeConfig {
        diameter: 30,
        ..StrokeConfig::default()
    };
    let mut doc = original.clone();
    let mut stroke = Stroke::begin_tone(&doc, "0", cfg, ToneOptions::default()).expect("begin");
    doc.layers[0].rect.right -= 1;
    let moved = doc.clone();
    assert!(!stroke.sample(&mut doc, sample(40.0, 40.0)));
    assert!(doc == moved);
}
