use super::*;
use pictura_core::{LayerBlock, VectorFillRule, VectorMask, VectorSubpath};

const VECTOR_MASK_FIXTURE: &[u8] =
    include_bytes!("../../../pictura-codec/tests/fixtures/vector_mask.psd");

fn vector_mask_doc() -> Document {
    pictura_codec::read_psd(VECTOR_MASK_FIXTURE).expect("vector_mask.psd parses")
}

#[test]
fn fixture_decodes_vector_masks() {
    let d = vector_mask_doc();

    let shape = d
        .layers
        .iter()
        .find(|l| l.name == "Shape")
        .expect("Shape layer");
    let mask = shape.vector_mask.as_ref().expect("Shape vector mask");
    assert!(!mask.invert, "Shape flags bit 0 is clear");
    assert!(!mask.disabled);
    assert_eq!(mask.subpaths.len(), 1, "one closed subpath");
    let sub = &mask.subpaths[0];
    assert!(sub.closed);
    assert_eq!(sub.operation, 1);
    assert_eq!(sub.fill_rule, VectorFillRule::EvenOdd);
    // The authored (1,1)-(3,3) rectangle, in 1/256 document pixels.
    assert!(sub.points.contains(&[256, 256]), "{:?}", sub.points);
    assert!(sub.points.contains(&[768, 768]), "{:?}", sub.points);

    let inverted = d
        .layers
        .iter()
        .find(|l| l.name == "Shape Inverted")
        .expect("Shape Inverted layer");
    let mask = inverted.vector_mask.as_ref().expect("inverted vector mask");
    assert!(mask.invert, "invert flag comes from flags bit 0");
}

#[test]
fn fixture_vector_mask_clips_fill_layers() {
    let out = composite_rgba(&vector_mask_doc());

    assert_eq!(rgb(&out, 1, 1), [255, 0, 0], "inside the Shape rectangle");
    assert_eq!(rgb(&out, 2, 2), [255, 0, 0]);
    assert_eq!(
        rgb(&out, 0, 0),
        [200, 100, 50],
        "the inverted fill is hidden inside its rectangle"
    );
    assert_eq!(rgb(&out, 4, 4), [200, 100, 50]);
    assert_eq!(
        rgb(&out, 6, 0),
        [0, 0, 255],
        "outside the inverted rectangle shows the blue fill"
    );
    assert_eq!(rgb(&out, 7, 7), [0, 0, 255]);
}

/// A minimal `vmsk` payload: version/flags header then one 26-byte record.
fn vmsk(version: u32, selector: u16) -> Vec<u8> {
    let mut data = version.to_be_bytes().to_vec();
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&selector.to_be_bytes());
    data.extend_from_slice(&[0u8; 24]);
    data
}

fn round_trip_with_vmsk(data: Vec<u8>) -> Layer {
    let mut layer = solid("L", full(2, 2), (10, 20, 30), 255, BlendMode::Normal, 255);
    layer.extra_blocks.push(LayerBlock {
        key: *b"vmsk",
        data,
    });
    let bytes = pictura_codec::write_psd(&doc(2, 2, vec![layer])).expect("writes");
    pictura_codec::read_psd(&bytes).expect("re-reads").layers[0].clone()
}

#[test]
fn malformed_vmsk_is_unset_and_never_panics() {
    for (label, data) in [
        ("a version other than 3", vmsk(2, 0)),
        ("an unknown selector", vmsk(3, 9)),
    ] {
        let layer = round_trip_with_vmsk(data.clone());
        assert!(layer.vector_mask.is_none(), "{label} leaves the view unset");
        let raw = layer
            .extra_block(b"vmsk")
            .expect("the raw block is preserved");
        assert_eq!(raw.data, data, "{label}: raw block re-emitted verbatim");
    }
}

#[test]
fn mask_alpha_multiplies_raster_and_vector() {
    let mut layer = solid("L", full(2, 2), (0, 0, 0), 255, BlendMode::Normal, 255);
    layer.mask = Some(LayerMask {
        rect: full(2, 2),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(vec![200; 4].into()),
        ..Default::default()
    });
    layer.vector_mask = Some(VectorMask {
        subpaths: vec![VectorSubpath {
            closed: true,
            operation: 1,
            fill_rule: VectorFillRule::EvenOdd,
            points: vec![[0, 0], [256, 0], [256, 256], [0, 256]],
        }],
        invert: false,
        disabled: false,
        ..Default::default()
    });

    assert_eq!(
        mask_alpha(&layer, 0, 0),
        200,
        "raster x full vector coverage"
    );
    assert_eq!(
        mask_alpha(&layer, 1, 1),
        0,
        "zero vector coverage suppresses"
    );

    layer.vector_mask.as_mut().unwrap().disabled = true;
    assert_eq!(
        mask_alpha(&layer, 1, 1),
        200,
        "a disabled vector mask changes nothing"
    );
}

#[test]
fn a_shape_layer_is_a_fill_cut_to_its_outline_and_survives_a_save() {
    let white = solid(
        "Background",
        full(8, 8),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let mut d = doc(8, 8, vec![white]);
    let options =
        pictura_core::shape::ShapeOptions::new(pictura_core::shape::ShapeKind::Rectangle, 0.0, 3);
    let outline = pictura_core::shape::outline(options, (2.0, 2.0), (6.0, 6.0), false, false)
        .expect("a rectangle");
    let path = add_shape_layer(&mut d, "", [255, 0, 0, 255], "Rectangle", &outline, None);
    assert_eq!(
        resolve_path(&d, &path).expect("created").name,
        "Rectangle 1"
    );

    let out = composite_rgba(&d);
    assert_eq!(rgb(&out, 3, 3), [255, 0, 0], "inside the outline");
    assert_eq!(rgb(&out, 1, 1), [255, 255, 255], "outside it");
    assert_eq!(rgb(&out, 6, 6), [255, 255, 255]);

    let bytes = pictura_codec::write_psd(&d).expect("writes");
    let reread = pictura_codec::read_psd(&bytes).expect("re-reads");
    assert_eq!(composite_rgba(&reread), out, "the shape layer round-trips");
}
