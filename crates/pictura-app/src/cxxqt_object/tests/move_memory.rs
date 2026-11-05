//! A Move on a canvas-sized layer around a small mark works at the mark's
//! size (#1207): the moving image, the planes it reads, and the commit's
//! damage.

use super::super::helpers_composite::{layer_buffer, layer_buffer_within};
use super::super::impl_transform::translate_move_group;
use super::pixel_layer;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack, PsdRect};

/// A `w × h` document whose one layer covers the canvas, transparent except
/// for an opaque `mark` with a pattern in its colour planes.
fn marked(w: u32, h: u32, mark: PsdRect) -> Document {
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel_layer("mark", w, h, (0, 0, 0));
    for c in &mut layer.channels {
        for (i, v) in c.data.iter_mut().enumerate() {
            let (x, y) = ((i as u32 % w) as i32, (i as u32 / w) as i32);
            let inside = x >= mark.left && x < mark.right && y >= mark.top && y < mark.bottom;
            *v = match (c.id, inside) {
                (-1, inside) => inside as u8 * 255,
                (_, true) => ((x * 7 + y * 13 + c.id as i32 * 31) % 256) as u8,
                (_, false) => 0,
            };
        }
    }
    doc.layers = LayerStack::from_layers(vec![layer]);
    doc
}

const MARK: PsdRect = PsdRect {
    top: 70,
    left: 20,
    bottom: 150,
    right: 61,
};

#[test]
fn a_bounded_buffer_is_the_crop_of_the_whole() {
    let doc = marked(90, 300, MARK);
    let layer = &doc.layers[0];
    let whole = layer_buffer(layer).expect("whole");
    let part = layer_buffer_within(layer, MARK).expect("part");
    assert_eq!((part.width, part.height, part.channels), (41, 80, 4));
    let (w, h) = (90usize, 300usize);
    let (pw, ph) = (41usize, 80usize);
    for k in 0..4 {
        for y in 0..ph {
            for x in 0..pw {
                let got = part.data[k * pw * ph + y * pw + x];
                let want = whole.data[k * w * h + (y + 70) * w + x + 20];
                assert_eq!(got, want, "plane {k} at ({x}, {y})");
            }
        }
    }
    let outside = PsdRect {
        top: 400,
        left: 0,
        bottom: 500,
        right: 10,
    };
    assert!(layer_buffer_within(layer, outside).is_none());
}

#[test]
fn reading_a_diverged_banded_layer_builds_no_mirror() {
    let mut doc = marked(90, 300, MARK);
    let history = doc.clone();
    for c in &mut doc.layers[0].channels {
        c.data.fracture(90);
        if let Some(band) = c.data.write_band(1) {
            band[0] = band[0].wrapping_add(1);
        }
    }
    let expected = {
        let mut flat = doc.clone();
        for c in &mut flat.layers[0].channels {
            c.data = c.data.to_vec().into();
        }
        layer_buffer_within(&flat.layers[0], MARK).expect("flat")
    };
    let got = layer_buffer_within(&doc.layers[0], MARK).expect("banded");
    assert_eq!(got.data.as_slice(), expected.data.as_slice());
    assert!(
        doc.layers[0].channels.iter().all(|c| !c.data.is_mirrored()),
        "a plane was materialized whole"
    );
    drop(history);
}

#[test]
fn a_move_damages_the_mark_before_and_after() {
    let mut doc = marked(90, 300, MARK);
    let dirty = translate_move_group(&mut doc, &["0".to_string()], 10, 30).expect("moved");
    assert_eq!(
        dirty,
        PsdRect {
            top: 70,
            left: 20,
            bottom: 180,
            right: 71,
        },
        "the damage is the mark's box before and after, not the layer's rect"
    );
    // A fully opaque layer still damages its whole rect.
    let mut solid = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    solid.layers = LayerStack::from_layers(vec![pixel_layer("solid", 8, 8, (1, 2, 3))]);
    let dirty = translate_move_group(&mut solid, &["0".to_string()], 2, 0).expect("moved");
    assert_eq!((dirty.left, dirty.right), (0, 10));
}
