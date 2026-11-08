use super::*;
use crate::focus::FocusOptions;
use crate::smudge::SmudgeOptions;
use crate::tone::ToneOptions;

#[test]
fn a_stroke_forks_only_touched_tiles() {
    let (w, h) = (8usize, 300usize);
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
    layer.channels[0].data.tile(w, h);
    let holder = layer.channels[0].data.clone();

    let ch: PlaneIndex = [Some(0), None, None, None];
    let mut scratch = Plane::filled(w, h, 1, 0u8);
    let mut applied = Plane::filled(w, h, 1, 0u8);
    let cells = crate::stroke::raster::cells(
        &mut layer,
        ch,
        w,
        h,
        0..w,
        0..64,
        &mut scratch,
        &mut applied,
    );
    assert_eq!(cells.len(), 1, "one tile under the rows");
    for mut cell in cells {
        if let Some(plane) = cell.planes[0].as_mut() {
            plane[0] = 9;
        }
    }

    assert!(
        !layer.channels[0].data.shares_tile(0, &holder),
        "the touched tile forked"
    );
    assert!(
        layer.channels[0].data.shares_tile(1, &holder),
        "the untouched tile stayed shared"
    );
    assert_eq!(holder.sample(0), Some(1));
    assert_eq!(scratch.tile_state(1), Some(pictura_core::TileState::Empty));
}

// [tsr_first_dab]
#[test]
fn a_tone_stroke_forks_only_touched_tiles() {
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 600,
        right: 64,
    };
    let mut doc = offset_layer_doc(64, 600, rect, (120, 90, 60, 255));
    for channel in &mut doc.layers[0].channels {
        channel.data.tile(64, 600);
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
        assert!(live.data.is_tiled(), "the plane was flattened");
        assert!(
            live.data.shares_tile(2, &held.data),
            "an untouched tile was copied"
        );
    }
    let red = &doc.layers[0].channels[0].data;
    assert!(!red.shares_tile(0, &history.layers[0].channels[0].data));
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

// [tp_stroke]
#[test]
fn a_stroke_costs_the_tiles_it_covers_not_the_document_width() {
    let forked = |side: i32| {
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: side,
            right: side,
        };
        let mut doc = offset_layer_doc(side as u32, side as u32, rect, (120, 90, 60, 255));
        for channel in &mut doc.layers[0].channels {
            channel.data.tile(side as usize, side as usize);
        }
        let history = doc.clone();
        let cfg = StrokeConfig {
            diameter: 40,
            ..StrokeConfig::default()
        };
        let mut stroke = Stroke::begin_at(&doc, "0", cfg).expect("begin");
        for i in 0..=10 {
            stroke.sample(&mut doc, sample(100.0 + 30.0 * i as f32, 100.0));
        }
        assert!(stroke.finish().is_some(), "the stroke painted");
        doc.layers[0]
            .channels
            .iter()
            .zip(&history.layers[0].channels)
            .map(|(live, held)| {
                (0..live.data.tile_count())
                    .filter(|&i| !live.data.shares_tile(i, &held.data))
                    .count()
            })
            .sum::<usize>()
    };
    let small = forked(1024);
    assert!(small > 0, "the stroke forked the tiles under it");
    assert_eq!(forked(4096), small, "the fork does not grow with the width");
}
