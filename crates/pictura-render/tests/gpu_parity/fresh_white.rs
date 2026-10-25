//! M44 E1: a freshly created white RGB document must composite to uniform
//! opaque white.

use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LockFlags, PixelBuffer,
    PsdRect,
};
use pictura_render::{composite_active, composite_rgba, Backend};

use crate::common::require_gpu;

/// A document shaped exactly like `PictureView::new_document(..., "white")`:
/// one full-size Normal layer whose colour planes and alpha are all 255.
fn fresh_white_document(w: u32, h: u32) -> Document {
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data.fill(255);
    let n = (w as usize) * (h as usize);
    let plane = || vec![255u8; n];
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        name: "Layer 0".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: plane().into(),
            },
            Channel {
                id: 1,
                data: plane().into(),
            },
            Channel {
                id: 2,
                data: plane().into(),
            },
            Channel {
                id: -1,
                data: plane().into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }]);
    doc
}

/// Assert every channel of every pixel is 255 in the planar straight-alpha
/// RGBA buffer.
fn assert_uniform_white(buffer: &PixelBuffer, ctx: &str) {
    assert_eq!(buffer.channels, 4, "{ctx}: expected straight RGBA");
    for (i, &byte) in buffer.data.iter().enumerate() {
        assert_eq!(byte, 255, "{ctx}: byte {i} is not opaque white");
    }
}

const W: u32 = 37;
const H: u32 = 23;

/// A freshly created white RGB document must composite to a uniform opaque-white
/// frame on the CPU, and on the GPU on both the first and a repeated composite.
/// An odd width/height guards against any row-stride assumption in the
/// readback.
#[test]
fn fresh_white_document_composites_uniform_white_on_cpu() {
    let cpu = composite_rgba(&fresh_white_document(W, H));
    assert_uniform_white(&cpu, "cpu");
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn fresh_white_document_composites_uniform_white_twice() {
    require_gpu();
    let doc = fresh_white_document(W, H);
    let (first, backend) = composite_active(&doc, true);
    assert_eq!(
        backend,
        Backend::Gpu,
        "fresh-white composite must run on the GPU"
    );
    assert_uniform_white(&first, "gpu first");
    let (second, _) = composite_active(&doc, true);
    assert_uniform_white(&second, "gpu second");
    assert_eq!(
        first.data, second.data,
        "a repeated composite must equal the first frame"
    );
}
