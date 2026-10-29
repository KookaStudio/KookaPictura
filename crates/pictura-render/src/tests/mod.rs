use super::*;
use pictura_adjust::{Adjustment, BrightnessContrastParams, HueSaturationParams, LevelsParams};
use pictura_codec::{write_descriptor, DescValue};
use pictura_core::{
    AdjustmentData, BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer,
    LayerMask, LockFlags, PixelBuffer, PsdRect,
};

mod adjustment;
mod blend;
mod clipping;
pub(crate) mod composite;
mod gradient_fill;
mod layer_effects;
mod native_depth;
mod pattern_fill;
mod raster_import;
mod rasterize;
mod region;
mod smart_object;
mod smart_object_actions;
mod text_gamma;
mod vector_fill;
mod vector_mask;

pub(crate) fn rect(top: i32, left: i32, bottom: i32, right: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

pub(crate) fn full(w: u32, h: u32) -> PsdRect {
    rect(0, 0, h as i32, w as i32)
}

pub(crate) fn solid(
    name: &str,
    r: PsdRect,
    rgb: (u8, u8, u8),
    alpha: u8,
    blend: BlendMode,
    opacity: u8,
) -> Layer {
    let w = r.width().max(0) as usize;
    let h = r.height().max(0) as usize;
    let n = w * h;
    Layer {
        name: name.into(),
        rect: r,
        blend,
        opacity,
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
                data: vec![rgb.0; n].into(),
            },
            Channel {
                id: 1,
                data: vec![rgb.1; n].into(),
            },
            Channel {
                id: 2,
                data: vec![rgb.2; n].into(),
            },
            Channel {
                id: -1,
                data: vec![alpha; n].into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

fn group(
    name: &str,
    blend: BlendMode,
    opacity: u8,
    mask: Option<LayerMask>,
    children: Vec<Layer>,
) -> Layer {
    Layer {
        name: name.into(),
        rect: rect(0, 0, 0, 0),
        blend,
        opacity,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask,
        adjustment: None,
        channels: Vec::new(),
        children: pictura_core::LayerStack::from_layers(children),
        is_group: true,
        background: false,
        ..Default::default()
    }
}

pub(crate) fn doc(w: u32, h: u32, layers: Vec<Layer>) -> Document {
    let mut d = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    d.layers = pictura_core::LayerStack::from_layers(layers);
    d
}

fn px(buf: &PixelBuffer, x: u32, y: u32) -> [u8; 4] {
    let plane = (buf.width * buf.height) as usize;
    let i = (y * buf.width + x) as usize;
    [
        buf.data[i],
        buf.data[plane + i],
        buf.data[2 * plane + i],
        buf.data[3 * plane + i],
    ]
}

fn assert_blend(mode: BlendMode, cb: [f32; 3], cs: [f32; 3], expected: [f32; 3]) {
    let got = blend(mode, cb, cs);
    for i in 0..3 {
        assert!(
            (got[i] - expected[i]).abs() <= 1.0 / 255.0 + 1e-6,
            "{mode:?}[{i}]: got {} want {}",
            got[i],
            expected[i]
        );
    }
}

fn adjdata(key: [u8; 4], data: Vec<u8>) -> AdjustmentData {
    AdjustmentData {
        key,
        data: data.into(),
    }
}

fn adjustment_layer(
    name: &str,
    key: [u8; 4],
    data: Vec<u8>,
    opacity: u8,
    mask: Option<LayerMask>,
) -> Layer {
    Layer {
        name: name.into(),
        rect: rect(0, 0, 0, 0),
        blend: BlendMode::Normal,
        opacity,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask,
        adjustment: Some(adjdata(key, data)),
        channels: Vec::new(),
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

fn rgb(buf: &PixelBuffer, x: u32, y: u32) -> [u8; 3] {
    let p = px(buf, x, y);
    [p[0], p[1], p[2]]
}

/// The psd-tools-authored pattern fixture: a `Patt` block with a 2x2 RGB
/// pattern (`pictura-pattern`) plus a document-sized `PtFl` layer referencing
/// it.
const PATTERN_FIXTURE: &[u8] =
    include_bytes!("../../../pictura-codec/tests/fixtures/pattern_fill.psd");

fn pattern_fixture_doc() -> Document {
    pictura_codec::read_psd(PATTERN_FIXTURE).expect("pattern_fill.psd parses")
}

/// The fixture pattern's 2x2 cells, row-major: red, green / blue, white.
const FIXTURE_TILE: [[[u8; 3]; 2]; 2] =
    [[[255, 0, 0], [0, 255, 0]], [[0, 0, 255], [255, 255, 255]]];

/// A `PtFl` descriptor for `params`, encoded the way the renderer decodes it.
fn ptfl(params: &PatternFillParams) -> AdjustmentData {
    let ptrn = DescValue::Object {
        name: String::new(),
        class_id: b"Ptrn".to_vec(),
        items: vec![(
            b"Idnt".to_vec(),
            DescValue::Text(format!("{}\0", params.pattern_id)),
        )],
    };
    let mut items = vec![
        (b"Ptrn".to_vec(), ptrn),
        (b"Scl ".to_vec(), DescValue::Double(params.scale as f64)),
        (b"Algn".to_vec(), DescValue::Bool(params.link_with_layer)),
    ];
    if params.origin != (0, 0) {
        items.push((
            b"phase".to_vec(),
            DescValue::Object {
                name: String::new(),
                class_id: b"Pnt ".to_vec(),
                items: vec![
                    (b"Hrzn".to_vec(), DescValue::Double(params.origin.0 as f64)),
                    (b"Vrtc".to_vec(), DescValue::Double(params.origin.1 as f64)),
                ],
            },
        ));
    }
    adjdata(
        *b"PtFl",
        write_descriptor(&DescValue::Object {
            name: String::new(),
            class_id: b"PtFl".to_vec(),
            items,
        }),
    )
}
