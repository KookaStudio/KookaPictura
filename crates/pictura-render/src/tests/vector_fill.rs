use super::*;
use pictura_adjust::{GradientKind, GradientStop};
use pictura_core::{AdjustmentData, LayerBlock};

const VECTOR_FILL_FIXTURE: &[u8] =
    include_bytes!("../../../pictura-codec/tests/fixtures/vector_fill.psd");

fn vector_fill_doc() -> Document {
    pictura_codec::read_psd(VECTOR_FILL_FIXTURE).expect("vector_fill.psd parses")
}

/// A `vscg` block: the 4-byte fill key followed by the version-16 descriptor
/// bytes the matching fill encoder writes.
fn vscg(key: &[u8; 4], descriptor: &[u8]) -> LayerBlock {
    let mut data = key.to_vec();
    data.extend_from_slice(descriptor);
    LayerBlock {
        key: *b"vscg",
        data: data.into(),
    }
}

/// A channel-less shape layer carrying `fill` in its `vscg` block.
fn shaped_layer(name: &str, w: u32, h: u32, fill: LayerBlock) -> Layer {
    let mut layer = solid(name, full(w, h), (0, 0, 0), 0, BlendMode::Normal, 255);
    layer.channels.clear();
    layer.extra_blocks.push(fill);
    layer
}

#[test]
fn decode_vector_fill_reuses_the_solid_and_gradient_decoders() {
    let solid_data = encode_solid_color_fill([10, 20, 30]);
    let block = vscg(b"SoCo", &solid_data.data);
    assert_eq!(
        crate::fill::decode_vector_fill(&block.data),
        Some(Adjustment::SolidFill([10, 20, 30, 255]))
    );

    let stops = [
        GradientStop {
            location: 0,
            color: [0, 0, 0],
        },
        GradientStop {
            location: 4096,
            color: [255, 255, 255],
        },
    ];
    let gradient = encode_gradient_fill(GradientKind::Linear, &stops, 0.0);
    let block = vscg(b"GdFl", &gradient.data);
    let Some(Adjustment::GradientFill(params)) = crate::fill::decode_vector_fill(&block.data)
    else {
        panic!("gradient vscg decodes");
    };
    assert_eq!(params.kind, GradientKind::Linear);
    assert_eq!(params.stops, stops.to_vec());
}

#[test]
fn fixture_shape_layer_keeps_vscg_and_decodes_the_fill() {
    let doc = vector_fill_doc();
    let shape = doc
        .layers
        .iter()
        .find(|l| l.name == "Shape")
        .expect("Shape layer");
    assert!(
        shape.extra_block(b"vscg").is_some(),
        "the raw vscg block stays in extra_blocks"
    );
    assert_eq!(
        crate::fill::decode_layer_fill(shape),
        Some(Adjustment::SolidFill([255, 0, 0, 255]))
    );
}

#[test]
fn fixture_vector_fill_is_clipped_by_the_vector_mask() {
    let out = composite_rgba(&vector_fill_doc());

    assert_eq!(rgb(&out, 1, 1), [255, 0, 0], "inside the vmsk rectangle");
    assert_eq!(rgb(&out, 4, 4), [255, 0, 0]);
    assert_eq!(rgb(&out, 0, 0), [200, 100, 50], "outside shows the base");
    assert_eq!(rgb(&out, 5, 5), [200, 100, 50]);
    assert_eq!(rgb(&out, 7, 7), [200, 100, 50]);
}

#[test]
fn vector_fill_without_a_mask_covers_the_layer_rect() {
    let fill = vscg(b"SoCo", &encode_solid_color_fill([0, 200, 0]).data);
    let mut shape = shaped_layer("shape", 4, 4, fill);
    shape.rect = rect(2, 2, 6, 6);
    let out = composite_rgba(&doc(8, 8, vec![shape]));

    assert_eq!(rgb(&out, 2, 2), [0, 200, 0], "top-left of the rect");
    assert_eq!(rgb(&out, 5, 5), [0, 200, 0], "bottom-right of the rect");
    assert_eq!(px(&out, 1, 1)[3], 0, "outside the rect stays transparent");
}

#[test]
fn adjustment_block_takes_precedence_over_vector_fill() {
    let fill = vscg(b"SoCo", &encode_solid_color_fill([0, 255, 0]).data);
    let mut shape = shaped_layer("shape", 8, 8, fill);
    shape.adjustment = Some(encode_solid_color_fill([0, 0, 255]));
    let out = composite_rgba(&doc(8, 8, vec![shape]));

    assert_eq!(
        rgb(&out, 3, 3),
        [0, 0, 255],
        "the modeled adjustment block wins over the vscg fill"
    );
}

#[test]
fn undecodable_adjustment_block_wins_over_vector_fill() {
    let fill = vscg(b"SoCo", &encode_solid_color_fill([0, 255, 0]).data);
    let mut shape = shaped_layer("shape", 4, 4, fill);
    shape.adjustment = Some(AdjustmentData {
        key: *b"zzzz",
        data: Vec::new().into(),
    });
    let base = solid(
        "Base",
        full(4, 4),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    let with = doc(4, 4, vec![base.clone(), shape]);
    let without = doc(4, 4, vec![base]);

    assert_eq!(
        composite_rgba(&with).data,
        composite_rgba(&without).data,
        "an undecodable adjustment block is a no-op even with a decodable vscg"
    );
}

#[test]
fn channel_less_layer_with_malformed_vscg_paints_black_over_its_rect() {
    // ponytail: the "does not change the backdrop" guarantee is relative to the
    // bare channel-less layer. A channel-less layer with no decoded fill still
    // goes through composite_pixels, which paints an opaque black rect.
    let fill = vscg(b"SoCo", b"\x00\x00\x00\x10garbage");
    let shape = shaped_layer("shape", 4, 4, fill);
    let base = solid(
        "Base",
        full(4, 4),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    let out = composite_rgba(&doc(4, 4, vec![base, shape]));

    assert_eq!(
        rgb(&out, 2, 2),
        [0, 0, 0],
        "the channel-less layer paints opaque black over its rect"
    );
}

#[test]
fn malformed_vector_fill_is_a_no_op() {
    for data in [
        b"SoCo\x00\x00\x00\x10garbage".to_vec(),
        b"SoCo\x00\x00\x00\x09\x00\x00\x00\x00".to_vec(),
        b"\x00\x00\x00\x00".to_vec(),
    ] {
        let base = solid(
            "Base",
            full(4, 4),
            (10, 20, 30),
            255,
            BlendMode::Normal,
            255,
        );
        let mut pixel = base.clone();
        pixel.extra_blocks.push(LayerBlock {
            key: *b"vscg",
            data: data.into(),
        });
        let with = doc(4, 4, vec![base.clone(), pixel]);
        let without = doc(4, 4, vec![base]);
        assert_eq!(
            composite_rgba(&with).data,
            composite_rgba(&without).data,
            "a malformed vscg renders identically to none"
        );
    }
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn vector_fill_composites_on_the_gpu() {
    let fill = vscg(b"SoCo", &encode_solid_color_fill([1, 2, 3]).data);
    let shape = shaped_layer("shape", 4, 4, fill);
    let doc = doc(4, 4, vec![shape]);

    let gpu = composite_gpu(&doc).expect("the decoded vector fill composites on the GPU");
    let cpu = composite_rgba(&doc);
    assert!(gpu
        .data
        .iter()
        .zip(cpu.data.iter())
        .all(|(g, c)| g.abs_diff(*c) <= 1));
}
