use super::content_aware::{content_aware_scale, minimum_vertical_seam, ContentAwareOptions};
use super::transform::{transform_layer_with, LayerTransform};
use super::kernel::Interpolation;
use pictura_core::{BitDepth, Channel, ColorMode, Document, LayerStack, PsdRect, Samples, SourceChannels};
use std::sync::atomic::AtomicBool;

fn rect(width: i32, height: i32) -> PsdRect {
    PsdRect { left: 0, top: 0, right: width, bottom: height }
}

fn fixture(width: usize, height: usize, values: &[u8]) -> Document {
    let channels = vec![
        Channel { id: 0, data: values.to_vec().into() },
        Channel { id: 1, data: values.to_vec().into() },
        Channel { id: 2, data: values.to_vec().into() },
        Channel { id: -1, data: vec![255; width * height].into() },
    ];
    let layer = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        name: "fixture".into(), rect: rect(width as i32, height as i32), channels,
        ..Default::default()
    });
    let mut doc = Document::new(width as u32, height as u32, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![layer]);
    doc
}

fn run(doc: &mut Document, width: i32, height: i32, amount: u8, protect: Option<&[u8]>) -> bool {
    content_aware_scale(doc, "0", rect(width, height), ContentAwareOptions { amount, protect }, &AtomicBool::new(false))
}

fn samples(doc: &Document, id: i16) -> Vec<u8> {
    doc.layers[0].channels().iter().find(|c| c.id == id).unwrap().data.iter().copied().collect()
}

#[test]
fn content_aware_amount_endpoints_and_midpoint() {
    let base = fixture(4, 2, &[10, 20, 200, 210, 10, 20, 200, 210]);
    let mut ordinary = base.clone();
    let mut reference = base.clone();
    assert!(transform_layer_with(&mut reference, "0", LayerTransform {
        scale_x: 0.5, scale_y: 1.0, angle_radians: 0.0, dx: -1.0, dy: 0.0,
    }, Interpolation::Bilinear));
    let mut carved = base.clone();
    let mut middle = base;
    assert!(run(&mut ordinary, 2, 2, 0, None));
    assert!(run(&mut carved, 2, 2, 100, None));
    assert!(run(&mut middle, 2, 2, 50, None));
    let a = samples(&ordinary, 0);
    assert_eq!(a, samples(&reference, 0));
    let b = samples(&carved, 0);
    let c = samples(&middle, 0);
    assert_ne!(a, b);
    for ((a, b), c) in a.iter().zip(&b).zip(&c) {
        assert!((i16::from(*c) * 2 - i16::from(*a) - i16::from(*b)).abs() <= 1);
    }

    let mut transparent = fixture(4, 2, &[10, 20, 200, 210, 10, 20, 200, 210]);
    pictura_codec::edit_layer(transparent.layers[0].as_mut(), |parts| {
        parts.channels.iter_mut().find(|c| c.id == -1).unwrap().data =
            vec![255, 64, 192, 0, 255, 64, 192, 0].into();
    });
    let mut uniform = transparent.clone();
    let mut seams = transparent.clone();
    let mut blend = transparent;
    assert!(run(&mut uniform, 2, 2, 0, None));
    assert!(run(&mut seams, 2, 2, 100, None));
    assert!(run(&mut blend, 2, 2, 50, None));
    let (uc, ua) = (samples(&uniform, 0), samples(&uniform, -1));
    let (sc, sa) = (samples(&seams, 0), samples(&seams, -1));
    let (bc, ba) = (samples(&blend, 0), samples(&blend, -1));
    for i in 0..bc.len() {
        let sum_alpha = f32::from(ua[i]) + f32::from(sa[i]);
        assert!((2 * i16::from(ba[i]) - i16::from(ua[i]) - i16::from(sa[i])).abs() <= 1);
        if sum_alpha > 0.0 {
            let expected = (f32::from(uc[i]) * f32::from(ua[i])
                + f32::from(sc[i]) * f32::from(sa[i])) / sum_alpha;
            assert!((f32::from(bc[i]) - expected).abs() <= 2.0);
        }
    }
}

#[test]
fn content_aware_protect_biases_seams() {
    let intensity = [0.5; 6];
    let mask = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let cancel = AtomicBool::new(false);
    assert_eq!(minimum_vertical_seam(&intensity, None, 3, 2, &cancel).unwrap(), vec![0, 0]);
    assert_eq!(minimum_vertical_seam(&intensity, Some(&mask), 3, 2, &cancel).unwrap(), vec![1, 1]);
    assert_eq!(mask, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    let mut doc = fixture(3, 2, &[42; 6]);
    pictura_codec::edit_layer(doc.layers[0].as_mut(), |parts| {
        parts.channels.iter_mut().find(|c| c.id == -1).unwrap().data =
            vec![50, 100, 150, 50, 100, 150].into();
    });
    doc.channels.push(Channel { id: 4, data: vec![255, 0, 0, 255, 0, 0].into() });
    let original_channel = doc.channels[0].clone();
    let coverage = doc.channels[0].data.to_vec();
    let mut plain = doc.clone();
    assert!(run(&mut plain, 2, 2, 100, None));
    assert!(run(&mut doc, 2, 2, 100, Some(&coverage)));
    assert_eq!(samples(&plain, -1), vec![100, 150, 100, 150]);
    assert_eq!(samples(&doc, -1), vec![50, 150, 50, 150]);
    assert_eq!(doc.channels[0], original_channel);
}

#[test]
fn content_aware_axes_and_enlargement() {
    let original = fixture(3, 2, &[10, 20, 30, 40, 50, 60]);
    for (width, height) in [(3, 2), (2, 2), (3, 1), (5, 2), (3, 4)] {
        let mut a = original.clone();
        let mut b = original.clone();
        assert!(run(&mut a, width, height, 100, None));
        assert!(run(&mut b, width, height, 100, None));
        assert_eq!(a, b);
        if (width, height) == (3, 2) { assert_eq!(a, original); }
        assert_eq!(a.layers[0].rect, rect(width, height));
        for channel in a.layers[0].channels() {
            assert_eq!(channel.data.len(), (width * height) as usize);
        }
        assert_eq!(samples(&a, 0), samples(&a, 1));
        assert_eq!(samples(&a, 0), samples(&a, 2));
        assert!(samples(&a, -1).iter().all(|&alpha| alpha == 255));
    }
    let mut moved = original.clone();
    let dest = PsdRect { left: 2, top: 1, right: 5, bottom: 3 };
    assert!(content_aware_scale(&mut moved, "0", dest,
        ContentAwareOptions { amount: 100, protect: None }, &AtomicBool::new(false)));
    for id in [-1, 0, 1, 2] { assert_eq!(samples(&moved, id), samples(&original, id)); }
    assert_eq!(moved.layers[0].rect, dest);
}

#[test]
fn content_aware_native_components() {
    for depth in [BitDepth::Sixteen, BitDepth::ThirtyTwo] {
        let mut fresh = fixture(4, 2, &[10, 20, 30, 40, 50, 60, 70, 80]);
        fresh.depth = depth;
        assert!(fresh.layers[0].source_channels.is_none());
        assert!(run(&mut fresh, 2, 2, 100, None));
        assert_eq!(fresh.source.depth(), Some(depth));
        let store = fresh.layers[0].source_channels.as_ref().unwrap();
        assert_eq!(store.depth, depth);
        assert!(store.planes.iter().all(|(_, plane)| plane.len() == 4));
    }
    for (depth, planes) in [
        (BitDepth::Sixteen, vec![(0, Samples::U16(vec![0, 1000, 2000, 3000].into())), (1, Samples::U16(vec![0, 2000, 4000, 6000].into())), (2, Samples::U16(vec![0, 3000, 6000, 9000].into())), (3, Samples::U16(vec![0, 4000, 8000, 12000].into()))]),
        (BitDepth::ThirtyTwo, vec![(0, Samples::F32(vec![-2.0, 2.0, 4.0, -8.0].into())), (1, Samples::F32(vec![-4.0, 4.0, 8.0, -16.0].into())), (2, Samples::F32(vec![-6.0, 6.0, 12.0, -24.0].into())), (3, Samples::F32(vec![-8.0, 8.0, 16.0, -32.0].into()))]),
    ] {
        let mut doc = fixture(4, 1, &[0, 20, 40, 60]);
        doc.mode = ColorMode::Cmyk;
        doc.depth = depth;
        doc.source = pictura_core::SourceState::new(Some(ColorMode::Cmyk), Some(depth), None, None).unwrap();
        doc.layers[0].source_channels = Some(SourceChannels::new(depth, rect(4, 1), planes));
        assert!(run(&mut doc, 3, 1, 100, None));
        let source = doc.layers[0].source_channels.as_ref().unwrap();
        assert_eq!(source.depth, depth);
        assert_eq!(source.planes[0].1.len(), 3);
        for (_, plane) in &source.planes {
            assert_eq!(plane.len(), 3);
        }
        match (&source.planes[0].1, &source.planes[1].1) {
            (Samples::U16(first), Samples::U16(second)) => {
                for (a, b) in first.iter().zip(second.iter()) {
                    assert_eq!(u32::from(*b), 2 * u32::from(*a));
                }
            }
            (Samples::F32(first), Samples::F32(second)) => {
                assert!(second.iter().any(|v| *v > 1.0));
                assert!(second.iter().any(|v| *v < 0.0));
                for (a, b) in first.iter().zip(second.iter()) {
                    assert_eq!(*b, 2.0 * *a);
                }
            }
            _ => panic!("native depth changed"),
        }
    }
}

#[test]
fn content_aware_invalid_is_atomic() {
    let original = fixture(4, 2, &[10, 20, 30, 40, 50, 60, 70, 80]);
    for (width, height) in [(0, 1), (-1, 1), (i32::MAX, i32::MAX)] {
        let mut doc = original.clone();
        assert!(!run(&mut doc, width, height, 100, None));
        assert_eq!(doc, original);
    }
    let mut converted = original.clone();
    converted.source = pictura_core::SourceState::new(Some(ColorMode::Cmyk), Some(BitDepth::Sixteen), None, None).unwrap();
    converted.depth = BitDepth::Sixteen;
    let before_converted = converted.clone();
    assert!(!run(&mut converted, 2, 2, 100, None));
    assert_eq!(converted, before_converted);
    let mut offset = original.clone();
    offset.layers[0].rect = PsdRect { left: -100, right: -96, top: 0, bottom: 2 };
    let before_offset = offset.clone();
    let invalid_dest = PsdRect { left: i32::MAX - 2, right: i32::MAX, top: 0, bottom: 2 };
    assert!(!content_aware_scale(&mut offset, "0", invalid_dest,
        ContentAwareOptions { amount: 100, protect: None }, &AtomicBool::new(false)));
    assert_eq!(offset, before_offset);
    let mut doc = original.clone();
    pictura_core::memory::set_host_for_test(Some(pictura_core::memory::HostReading {
        total: Some(1024 * 1024 * 1024),
        available: Some(0),
        rss: Some(1024 * 1024 * 1024),
        ..Default::default()
    }));
    pictura_core::memory::clear_refusal();
    assert!(!run(&mut doc, 2, 2, 100, None));
    assert!(pictura_core::memory::last_refusal().is_some());
    pictura_core::memory::set_host_for_test(None);
    pictura_core::memory::clear_refusal();
    assert_eq!(doc, original);
    let mut doc = original.clone();
    pictura_core::memory::fail_next_plane_for_test();
    assert!(!run(&mut doc, 2, 2, 100, None));
    assert!(pictura_core::memory::last_refusal().is_some());
    pictura_core::memory::clear_refusal();
    assert_eq!(doc, original);
    let mut doc = original.clone();
    assert!(!content_aware_scale(&mut doc, "0", rect(2, 2), ContentAwareOptions { amount: 100, protect: None }, &AtomicBool::new(true)));
    assert_eq!(doc, original);
}
