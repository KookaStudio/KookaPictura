use super::*;
use crate::spacing::SpacingMode;
use crate::tone::ToneOptions;
use pictura_core::{BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Layer, LockFlags, Plane};

const WHITE: (u8, u8, u8, u8) = (255, 255, 255, 255);
const RED: Rgba = Rgba {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};

fn layer(rect: PsdRect, rgba: (u8, u8, u8, u8)) -> Layer {
    let n = (rect.width().max(0) * rect.height().max(0)) as usize;
    let (r, g, b, a) = rgba;
    pictura_core::Layer::from_parts(pictura_core::LayerParts {
        name: "px".into(),
        rect,
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
                data: vec![r; n].into(),
            },
            Channel {
                id: 1,
                data: vec![g; n].into(),
            },
            Channel {
                id: 2,
                data: vec![b; n].into(),
            },
            Channel {
                id: -1,
                data: vec![a; n].into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }, pictura_core::LayerContent::Pixel)
}

fn doc(layer: Layer) -> Document {
    let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    let _ = doc.layers.push(layer);
    doc
}

fn selection(width: u32, height: u32, inside: impl Fn(i32, i32) -> bool) -> Coverage {
    let mut data = vec![0u8; (width * height) as usize];
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            if inside(x, y) {
                data[(y as usize) * width as usize + x as usize] = 255;
            }
        }
    }
    Coverage::for_canvas(&Plane::from(data), width, height).unwrap()
}

fn pixel(doc: &Document, x: i32, y: i32) -> (u8, u8, u8, u8) {
    let layer = &doc.layers[0];
    let rect = layer.rect;
    if x < rect.left || y < rect.top || x >= rect.right || y >= rect.bottom {
        return (0, 0, 0, 0);
    }
    let i = (y - rect.top) as usize * rect.width() as usize + (x - rect.left) as usize;
    let at = |id| {
        channel_data(layer, id)
            .and_then(|d| d.get(i).copied())
            .unwrap_or(0)
    };
    (at(0), at(1), at(2), at(-1))
}

fn brush(diameter: u32) -> StrokeConfig {
    StrokeConfig {
        color: RED,
        diameter,
        hardness: 100,
        spacing: SpacingMode::Fixed(100),
        opacity: 100,
        flow: 100,
        aliased: true,
        ..StrokeConfig::default()
    }
}

#[test]
fn a_selection_clip_stops_a_stroke_at_its_edge() {
    let mut doc = doc(layer(
        PsdRect {
            top: 0,
            left: 0,
            bottom: 8,
            right: 8,
        },
        WHITE,
    ));
    let mut stroke = Stroke::begin_at(&doc, "0", brush(8)).expect("stroke begins");
    stroke.set_coverage_clip(Some(selection(8, 8, |x, y| {
        (2..6).contains(&x) && (2..6).contains(&y)
    })));
    assert!(stroke.sample(
        &mut doc,
        StrokeSample {
            x: 4.0,
            y: 4.0,
            pressure: 1.0,
        }
    ));
    let _ = stroke.finish();

    assert_eq!(pixel(&doc, 4, 4), (255, 0, 0, 255), "inside is painted");
    assert_eq!(
        pixel(&doc, 1, 4),
        WHITE,
        "left of the selection is untouched"
    );
    assert_eq!(
        pixel(&doc, 6, 4),
        WHITE,
        "right of the selection is untouched"
    );
}

#[test]
fn a_partial_selection_scales_the_paint() {
    let mut doc = doc(layer(
        PsdRect {
            top: 0,
            left: 0,
            bottom: 8,
            right: 8,
        },
        WHITE,
    ));
    let mut data = vec![0u8; 64];
    data[4 * 8 + 4] = 128;
    let clip = Coverage::for_canvas(&Plane::from(data), 8, 8).unwrap();
    let mut stroke = Stroke::begin_at(&doc, "0", brush(8)).expect("stroke begins");
    stroke.set_coverage_clip(Some(clip));
    assert!(stroke.sample(
        &mut doc,
        StrokeSample {
            x: 4.0,
            y: 4.0,
            pressure: 1.0,
        }
    ));
    let _ = stroke.finish();

    let (r, g, b, a) = pixel(&doc, 4, 4);
    assert_eq!((r, a), (255, 255), "red over white keeps red and alpha");
    assert_eq!(g, b, "the partial coverage moves both channels equally");
    assert!(
        (1..255).contains(&g),
        "half coverage lands between white and red, got {g}"
    );
}

#[test]
fn a_stroke_without_a_selection_paints_everywhere() {
    let mut doc = doc(layer(
        PsdRect {
            top: 0,
            left: 0,
            bottom: 8,
            right: 8,
        },
        WHITE,
    ));
    let mut stroke = Stroke::begin_at(&doc, "0", brush(8)).expect("stroke begins");
    assert!(stroke.sample(
        &mut doc,
        StrokeSample {
            x: 4.0,
            y: 4.0,
            pressure: 1.0,
        }
    ));
    let _ = stroke.finish();

    assert_eq!(pixel(&doc, 4, 4), (255, 0, 0, 255));
    assert_eq!(pixel(&doc, 1, 4), (255, 0, 0, 255));
    assert_eq!(pixel(&doc, 7, 4), (255, 0, 0, 255));
}

#[test]
fn a_per_dab_tool_is_clipped_too() {
    let mut doc = doc(layer(
        PsdRect {
            top: 0,
            left: 0,
            bottom: 8,
            right: 8,
        },
        (128, 128, 128, 255),
    ));
    let mut stroke = Stroke::begin_tone(&doc, "0", brush(8), ToneOptions::default())
        .expect("tone stroke begins");
    stroke.set_coverage_clip(Some(selection(8, 8, |x, y| {
        (2..6).contains(&x) && (2..6).contains(&y)
    })));
    assert!(stroke.sample(
        &mut doc,
        StrokeSample {
            x: 4.0,
            y: 4.0,
            pressure: 1.0,
        }
    ));
    let _ = stroke.finish();

    assert_ne!(
        pixel(&doc, 4, 4),
        (128, 128, 128, 255),
        "the tone stroke changed the selected centre"
    );
    assert_eq!(
        pixel(&doc, 1, 4),
        (128, 128, 128, 255),
        "the tone stroke left the unselected pixel alone"
    );
}

#[test]
fn a_clip_follows_the_layer_document_position() {
    let mut doc = doc(layer(
        PsdRect {
            top: 4,
            left: 4,
            bottom: 12,
            right: 12,
        },
        WHITE,
    ));
    let mut stroke = Stroke::begin_at(&doc, "0", brush(8)).expect("stroke begins");
    stroke.set_coverage_clip(Some(selection(16, 16, |x, y| {
        (6..10).contains(&x) && (6..10).contains(&y)
    })));
    assert!(stroke.sample(
        &mut doc,
        StrokeSample {
            x: 8.0,
            y: 8.0,
            pressure: 1.0,
        }
    ));
    let _ = stroke.finish();

    assert_eq!(
        pixel(&doc, 8, 8),
        (255, 0, 0, 255),
        "the layer-local centre"
    );
    assert_eq!(
        pixel(&doc, 5, 8),
        WHITE,
        "a layer pixel left of the document-space selection is untouched"
    );
    assert_eq!(
        pixel(&doc, 11, 8),
        WHITE,
        "a layer pixel right of the document-space selection is untouched"
    );
}

#[test]
fn a_grown_layer_keeps_the_document_space_clip() {
    let empty = pictura_core::Layer::from_parts(pictura_core::LayerParts {
        name: "px".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        },
        channels: [0i16, 1, 2, -1]
            .into_iter()
            .map(|id| Channel {
                id,
                data: Plane::default(),
            })
            .collect(),
        ..pictura_core::LayerParts::from(layer(
            PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            },
            WHITE,
        ))
    }, pictura_core::LayerContent::Pixel);
    let mut doc = doc(empty);
    let mut stroke = Stroke::begin_at(&doc, "0", brush(6)).expect("stroke begins");
    stroke.set_coverage_clip(Some(selection(16, 16, |x, y| {
        (6..10).contains(&x) && (6..10).contains(&y)
    })));
    assert!(stroke.sample(
        &mut doc,
        StrokeSample {
            x: 8.0,
            y: 8.0,
            pressure: 1.0,
        }
    ));
    let _ = stroke.finish();

    assert!(
        doc.layers[0].rect.width() > 0 && doc.layers[0].rect.height() > 0,
        "the stroke grew the empty layer"
    );
    assert_eq!(pixel(&doc, 8, 8), (255, 0, 0, 255), "inside the selection");
    assert_eq!(
        pixel(&doc, 5, 8),
        (0, 0, 0, 0),
        "the grown layer is transparent outside the selection"
    );
}
