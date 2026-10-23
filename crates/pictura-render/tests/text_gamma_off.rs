//! Blend Text Colors Using Gamma turned off. Its own binary: the setting is
//! process wide, and the unit tests composite type with it on.

use pictura_core::{BitDepth, Channel, ColorMode, Document, Layer, LayerStack, PsdRect, TypeTool};

fn layer(rgb: u8, alpha: Vec<u8>, type_tool: Option<TypeTool>) -> Layer {
    let n = alpha.len();
    Layer {
        name: "layer".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 1,
            right: n as i32,
        },
        opacity: 255,
        fill: 255,
        visible: true,
        channels: [0, 1, 2]
            .into_iter()
            .map(|id| Channel {
                id,
                data: vec![rgb; n].into(),
            })
            .chain([Channel {
                id: -1,
                data: alpha.into(),
            }])
            .collect(),
        type_tool,
        ..Default::default()
    }
}

fn composite(type_tool: Option<TypeTool>) -> Vec<u8> {
    let mut doc = Document::new(256, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        layer(255, vec![255; 256], None),
        layer(0, (0..=255u8).collect(), type_tool),
    ]);
    pictura_render::composite_rgba(&doc).data.to_vec()
}

#[test]
fn off_reproduces_the_plain_blend() {
    let tool = TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        text: "Hi".into(),
        bounds: [0, 0, 1, 256],
        text_desc: Vec::new(),
        warp_desc: Vec::new(),
        fonts: Vec::new(),
        style: None,
        vertical: false,
        box_bounds: None,
        path: None,
    };
    assert_ne!(
        composite(Some(tool.clone())),
        composite(None),
        "on by default"
    );
    pictura_render::set_text_gamma(None);
    assert_eq!(pictura_render::text_gamma(), None);
    assert_eq!(composite(Some(tool)), composite(None));
}
