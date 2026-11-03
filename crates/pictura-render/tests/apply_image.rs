//! Image > Apply Image kernel (#609), against the public compositor and the
//! documented Add and Subtract arithmetic.

use std::sync::Arc;

use pictura_core::{
    BitDepth, BlendMode, Document, Layer, LayerMask, PsdRect, Samples, SourceChannels,
};
use pictura_render::{
    apply_image_planes, apply_image_region, composite_rgba, ApplyImageChannel, ApplyImageMode,
    ApplyImageParams, ApplyImagePlanes, ApplyImageTarget,
};

const LAYER: ApplyImageTarget = ApplyImageTarget {
    origin: (0, 0),
    gray: false,
    native: false,
};

fn solid(w: usize, h: usize, rgba: [u8; 4]) -> Document {
    let pixels: Vec<u8> = rgba.iter().copied().cycle().take(w * h * 4).collect();
    Document::from_rgba("t", w as u32, h as u32, &pixels)
}

fn whole(layer: &Layer) -> PsdRect {
    layer.rect
}

fn plane(layer: &Layer, id: i16) -> Vec<u8> {
    let channel = layer.channels.iter().find(|c| c.id == id).unwrap();
    channel.data.to_vec()
}

fn params(source: &Document, mode: ApplyImageMode) -> ApplyImageParams {
    ApplyImageParams {
        source: Arc::new(apply_image_planes(source, None, ApplyImageChannel::Composite).unwrap()),
        invert: false,
        mode,
        opacity: 1.0,
        preserve_transparency: false,
        mask: None,
        mask_invert: false,
    }
}

fn run(target: &mut Document, p: &ApplyImageParams, selection: Option<&LayerMask>) {
    let layer = target.layers.first_mut().unwrap();
    let rect = whole(layer);
    apply_image_region(layer, p, selection, rect, LAYER).unwrap();
}

fn first(target: &Document, id: i16) -> u8 {
    plane(target.layers.first().unwrap(), id)[0]
}

#[test]
fn add_scale_two_averages() {
    let mut target = solid(4, 4, [100, 20, 240, 255]);
    let source = solid(4, 4, [200, 60, 10, 255]);
    let mode = ApplyImageMode::Add {
        scale: 2.0,
        offset: 0.0,
    };
    run(&mut target, &params(&source, mode), None);
    assert!(first(&target, 0).abs_diff(150) <= 1);
    assert!(first(&target, 1).abs_diff(40) <= 1);
    assert!(first(&target, 2).abs_diff(125) <= 1);
}

#[test]
fn subtract_honours_scale_and_offset() {
    let source = solid(2, 2, [50, 50, 50, 255]);
    let result = |scale, offset| {
        let mut target = solid(2, 2, [200, 200, 200, 255]);
        let mode = ApplyImageMode::Subtract { scale, offset };
        run(&mut target, &params(&source, mode), None);
        first(&target, 0)
    };
    assert_eq!(result(1.0, 0.0), 150);
    assert_eq!(result(2.0, 0.0), 75);
    assert_eq!(result(1.0, 30.0), 180);
}

#[test]
fn layer_modes_match_the_compositor() {
    for mode in BlendMode::LAYER_MODES {
        if matches!(mode, BlendMode::Dissolve | BlendMode::Subtract) {
            continue;
        }
        let top = [190u8, 90, 40, 255];
        let mut stacked = solid(2, 2, [70, 160, 210, 255]);
        let mut upper = solid(2, 2, top).layers.first().unwrap().clone();
        upper.blend = mode;
        stacked.layers.push(upper).unwrap();
        let expected = composite_rgba(&stacked);

        let mut target = solid(2, 2, [70, 160, 210, 255]);
        let source = solid(2, 2, top);
        run(
            &mut target,
            &params(&source, ApplyImageMode::Layer(mode)),
            None,
        );
        for c in 0..3 {
            let want = expected.data[c * 4];
            assert!(
                first(&target, c as i16).abs_diff(want) <= 1,
                "{mode:?} channel {c}"
            );
        }
    }
}

#[test]
fn single_channel_feeds_every_target_channel() {
    let source = solid(2, 2, [200, 10, 30, 255]);
    let mut target = solid(2, 2, [1, 2, 3, 255]);
    let mut p = params(&source, ApplyImageMode::Layer(BlendMode::Normal));
    p.source = Arc::new(apply_image_planes(&source, None, ApplyImageChannel::Color(0)).unwrap());
    run(&mut target, &p, None);
    for c in 0..3 {
        assert_eq!(first(&target, c), 200);
    }
}

#[test]
fn opacity_and_invert() {
    let source = solid(2, 2, [200, 200, 200, 255]);
    let mut target = solid(2, 2, [100, 100, 100, 255]);
    let mut p = params(&source, ApplyImageMode::Layer(BlendMode::Normal));
    p.opacity = 0.5;
    run(&mut target, &p, None);
    assert!(first(&target, 0).abs_diff(150) <= 1);

    let mut target = solid(2, 2, [100, 100, 100, 255]);
    p.opacity = 1.0;
    p.invert = true;
    run(&mut target, &p, None);
    assert_eq!(first(&target, 0), 55);
}

#[test]
fn preserve_transparency_keeps_alpha() {
    let source = solid(2, 2, [200, 10, 30, 255]);
    let mut target = solid(2, 2, [1, 2, 3, 0]);
    let before = plane(target.layers.first().unwrap(), 0);
    let mut p = params(&source, ApplyImageMode::Layer(BlendMode::Normal));
    p.preserve_transparency = true;
    run(&mut target, &p, None);
    let layer = target.layers.first().unwrap();
    assert_eq!(plane(layer, 0), before);
    assert!(plane(layer, -1).iter().all(|&a| a == 0));
}

#[test]
fn clear_target_takes_the_source() {
    let source = solid(2, 2, [200, 10, 30, 255]);
    let mut target = solid(2, 2, [1, 2, 3, 0]);
    let p = params(&source, ApplyImageMode::Layer(BlendMode::Multiply));
    run(&mut target, &p, None);
    assert_eq!(first(&target, -1), 255);
    assert_eq!(first(&target, 0), 200);
    assert_eq!(first(&target, 1), 10);
}

#[test]
fn inverted_mask_applies_where_dark() {
    let source = solid(2, 1, [200, 200, 200, 255]);
    let mut target = solid(2, 1, [10, 10, 10, 255]);
    let mut p = params(&source, ApplyImageMode::Layer(BlendMode::Normal));
    let mask = ApplyImagePlanes::from_coverage(2, 1, vec![0u8, 255].into()).unwrap();
    p.mask = Some(Arc::new(mask));
    p.mask_invert = true;
    run(&mut target, &p, None);
    assert_eq!(plane(target.layers.first().unwrap(), 0), vec![200, 10]);
}

#[test]
fn selection_confines_the_result() {
    let source = solid(2, 1, [200, 200, 200, 255]);
    let mut target = solid(2, 1, [10, 10, 10, 255]);
    let selection = LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 1,
            right: 2,
        },
        data: Some(vec![255u8, 0].into()),
        ..Default::default()
    };
    let p = params(&source, ApplyImageMode::Layer(BlendMode::Normal));
    run(&mut target, &p, Some(&selection));
    assert_eq!(plane(target.layers.first().unwrap(), 0), vec![200, 10]);
}

#[test]
fn section_matches_full_apply() {
    let pixels: Vec<u8> = (0..64u32)
        .flat_map(|i| [i as u8 * 3, 40, 200, 255])
        .collect();
    let target = Document::from_rgba("t", 8, 8, &pixels);
    let source = solid(8, 8, [90, 140, 30, 255]);
    let p = params(&source, ApplyImageMode::Layer(BlendMode::Overlay));
    let mut full = target.clone();
    run(&mut full, &p, None);
    let mut part = target.clone();
    let section = PsdRect {
        top: 2,
        left: 3,
        bottom: 6,
        right: 7,
    };
    let layer = part.layers.first_mut().unwrap();
    apply_image_region(layer, &p, None, section, LAYER).unwrap();
    let (original, full, part) = (
        plane(target.layers.first().unwrap(), 0),
        plane(full.layers.first().unwrap(), 0),
        plane(part.layers.first().unwrap(), 0),
    );
    for y in 0..8 {
        for x in 0..8 {
            let inside = (2..6).contains(&y) && (3..7).contains(&x);
            let want = if inside { &full } else { &original };
            assert_eq!(part[y * 8 + x], want[y * 8 + x]);
        }
    }
}

fn native_layer(depth: BitDepth, planes: Vec<(i16, Samples)>) -> Document {
    let mut doc = solid(1, 1, [128, 128, 128, 255]);
    doc.source_depth = Some(depth);
    let layer = doc.layers.first_mut().unwrap();
    layer.source_channels = Some(SourceChannels::new(depth, layer.rect, planes));
    doc
}

const NATIVE: ApplyImageTarget = ApplyImageTarget {
    origin: (0, 0),
    gray: false,
    native: true,
};

#[test]
fn sixteen_bit_keeps_precision() {
    let planes = (0..3)
        .map(|id| (id, Samples::U16(vec![30000u16].into())))
        .collect();
    let mut doc = native_layer(BitDepth::Sixteen, planes);
    let source = solid(1, 1, [255, 255, 255, 255]);
    let mut p = params(&source, ApplyImageMode::Layer(BlendMode::Normal));
    p.opacity = 0.001;
    let layer = doc.layers.first_mut().unwrap();
    let rect = whole(layer);
    apply_image_region(layer, &p, None, rect, NATIVE).unwrap();
    let store = layer.source_channels.as_ref().unwrap();
    let Samples::U16(value) = &store.planes[0].1 else {
        panic!("store stays 16-bit");
    };
    assert!((30034..=30037).contains(&value[0]), "{}", value[0]);
    assert_eq!(plane(layer, 0)[0], (value[0] >> 8) as u8);
}

#[test]
fn thirty_two_bit_does_not_clamp() {
    let planes = (0..3)
        .map(|id| (id, Samples::F32(vec![0.75f32].into())))
        .collect();
    let mut doc = native_layer(BitDepth::ThirtyTwo, planes);
    let source = solid(1, 1, [255, 255, 255, 255]);
    let mode = ApplyImageMode::Add {
        scale: 1.0,
        offset: 0.0,
    };
    let p = params(&source, mode);
    let layer = doc.layers.first_mut().unwrap();
    let rect = whole(layer);
    apply_image_region(layer, &p, None, rect, NATIVE).unwrap();
    let store = layer.source_channels.as_ref().unwrap();
    let Samples::F32(value) = &store.planes[0].1 else {
        panic!("store stays 32-bit");
    };
    assert!((value[0] - 1.75).abs() < 1e-6);
    assert_eq!(plane(layer, 0)[0], 255);
}
