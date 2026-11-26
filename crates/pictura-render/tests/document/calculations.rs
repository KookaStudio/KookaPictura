//! Image > Calculations kernel (#610), against the shared blend, the
//! documented Add/Subtract arithmetic, the mask, and native source reads.

use std::sync::Arc;

use pictura_core::{BitDepth, BlendMode, Channel, Document, Samples, SourceChannels};
use pictura_render::{
    apply_image_planes, calculate, ApplyImageChannel, ApplyImageMode, ApplyImagePlanes,
    CalculationsParams,
};

fn solid(w: usize, h: usize, rgba: [u8; 4]) -> Document {
    let pixels: Vec<u8> = rgba.iter().copied().cycle().take(w * h * 4).collect();
    Document::from_rgba("t", w as u32, h as u32, &pixels)
}

fn planes(doc: &Document, channel: ApplyImageChannel) -> Arc<ApplyImagePlanes> {
    Arc::new(apply_image_planes(doc, None, channel).expect("readable planes"))
}

fn gray_planes(value: u8, w: usize, h: usize) -> Arc<ApplyImagePlanes> {
    planes(
        &solid(w, h, [value, value, value, 255]),
        ApplyImageChannel::Composite,
    )
}

fn params(a: Arc<ApplyImagePlanes>, b: Arc<ApplyImagePlanes>) -> CalculationsParams {
    CalculationsParams {
        source1: a,
        invert1: false,
        source2: b,
        invert2: false,
        mode: ApplyImageMode::Layer(BlendMode::Normal),
        opacity: 1.0,
        mask: None,
        mask_invert: false,
    }
}

fn run(p: &CalculationsParams) -> Vec<u8> {
    calculate(p).expect("matched sizes").to_vec()
}

fn narrow(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[test]
fn add_scale_two_averages() {
    let mut p = params(gray_planes(200, 2, 2), gray_planes(100, 2, 2));
    p.mode = ApplyImageMode::Add {
        scale: 2.0,
        offset: 0.0,
    };
    assert!(run(&p).iter().all(|&value| value.abs_diff(150) <= 1));
}

#[test]
fn subtract_honours_scale_and_offset() {
    let result = |scale, offset| {
        let mut p = params(gray_planes(200, 1, 1), gray_planes(50, 1, 1));
        p.mode = ApplyImageMode::Subtract { scale, offset };
        run(&p)[0]
    };
    assert_eq!(result(1.0, 0.0), 150);
    assert_eq!(result(2.0, 0.0), 75);
    assert_eq!(result(1.0, 30.0), 180);
}

#[test]
fn invert_applies_per_source() {
    let value = 64.0 / 255.0;
    let expected = |a: f32, b: f32| narrow(a * b);
    let run_with = |invert1, invert2| {
        let mut p = params(gray_planes(64, 1, 1), gray_planes(64, 1, 1));
        p.mode = ApplyImageMode::Layer(BlendMode::Multiply);
        p.invert1 = invert1;
        p.invert2 = invert2;
        run(&p)[0]
    };
    assert_eq!(run_with(false, false), expected(value, value));
    assert_eq!(run_with(true, false), expected(1.0 - value, value));
    assert_eq!(run_with(false, true), expected(value, 1.0 - value));
    assert_eq!(run_with(true, true), expected(1.0 - value, 1.0 - value));
}

#[test]
fn opacity_mixes_toward_source_one() {
    let a = 51.0 / 255.0;
    let b = 204.0 / 255.0;
    let mut p = params(gray_planes(51, 1, 1), gray_planes(204, 1, 1));
    p.opacity = 0.5;
    assert_eq!(run(&p)[0], narrow(a + (b - a) * 0.5));
}

#[test]
fn gray_matches_grayscale_conversion() {
    let doc = solid(1, 1, [100, 150, 200, 255]);
    let source = apply_image_planes(&doc, None, ApplyImageChannel::Composite).unwrap();
    let expected = 0.299 * (100.0 / 255.0) + 0.587 * (150.0 / 255.0) + 0.114 * (200.0 / 255.0);
    assert!((source.gray(0) - expected).abs() < 1e-5);

    // Rec.601 differs from the compositor's non-separable luminance on blue.
    let blue = solid(1, 1, [0, 0, 255, 255]);
    let source = apply_image_planes(&blue, None, ApplyImageChannel::Composite).unwrap();
    assert_eq!(narrow(source.gray(0)), 29);
}

#[test]
fn alpha_channel_as_source() {
    let mut doc = solid(2, 1, [10, 20, 30, 255]);
    doc.channels.push(Channel {
        id: 0,
        data: vec![200u8, 40].into(),
    });
    let alpha = planes(&doc, ApplyImageChannel::Alpha(0));
    let black = gray_planes(0, 2, 1);
    let p = params(black, alpha);
    assert_eq!(run(&p), vec![200, 40]);
}

fn native_sixteen(value: u16) -> Document {
    let mut doc = solid(1, 1, [128, 128, 128, 255]);
    doc.source
        .set_native(Some(BitDepth::Sixteen), doc.source.planes().clone())
        .expect("valid source depth");
    let layer = doc.layers.first_mut().unwrap();
    let mut planes = (0..3)
        .map(|id| (id, Samples::U16(vec![value].into())))
        .collect::<Vec<_>>();
    planes.push((-1, Samples::U16(vec![65535].into())));
    layer.source_channels = Some(SourceChannels::new(BitDepth::Sixteen, layer.rect, planes));
    doc
}

fn native_precision(value: u16) -> u8 {
    let doc = native_sixteen(value);
    let source = Arc::new(
        apply_image_planes(&doc, Some("0"), ApplyImageChannel::Composite)
            .expect("readable native layer"),
    );
    let mut p = params(source, gray_planes(0, 1, 1));
    p.opacity = 0.0;
    run(&p)[0]
}

#[test]
fn sixteen_bit_sources_keep_precision() {
    assert_eq!(native_precision(32767), 127);
    assert_eq!(native_precision(32768), 128);
}

#[test]
fn mask_hides() {
    let mut p = params(gray_planes(200, 2, 1), gray_planes(200, 2, 1));
    p.mask = Some(Arc::new(
        ApplyImagePlanes::from_coverage(2, 1, vec![0u8, 255].into()).unwrap(),
    ));
    assert_eq!(run(&p), vec![0, 200]);
}

#[test]
fn inverted_mask_reveals_dark() {
    let mut p = params(gray_planes(200, 2, 1), gray_planes(200, 2, 1));
    p.mask = Some(Arc::new(
        ApplyImagePlanes::from_coverage(2, 1, vec![0u8, 255].into()).unwrap(),
    ));
    p.mask_invert = true;
    assert_eq!(run(&p), vec![200, 0]);
}

#[test]
fn selection_as_mask() {
    let mut p = params(gray_planes(200, 2, 1), gray_planes(200, 2, 1));
    p.mask = Some(Arc::new(
        ApplyImagePlanes::from_coverage(2, 1, vec![255u8, 0].into()).unwrap(),
    ));
    assert_eq!(run(&p), vec![200, 0]);
}

#[test]
fn no_mask_applies_everywhere() {
    let p = params(gray_planes(0, 2, 1), gray_planes(200, 2, 1));
    assert_eq!(run(&p), vec![200, 200]);
}
