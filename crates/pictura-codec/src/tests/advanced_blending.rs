//! Advanced-blending (knko/clbl/infx + Blend If) read/write tests.

use super::*;

use crate::encode_blend_if;

/// Assemble a 1x1 RGB PSD whose sole layer carries the given blending-ranges
/// body and no extra tagged blocks.
fn ranges_layer_psd(ranges: &[u8]) -> Vec<u8> {
    let mut extra = Vec::new();
    extra.extend_from_slice(&0u32.to_be_bytes()); // mask data length
    extra.extend_from_slice(&(ranges.len() as u32).to_be_bytes());
    extra.extend_from_slice(ranges);
    extra.extend_from_slice(&[1, b'L', 0, 0]); // pascal name "L"
    if extra.len() % 2 == 1 {
        extra.push(0);
    }

    let mut rec = Vec::new();
    for v in [0i32, 0, 1, 1] {
        rec.extend_from_slice(&v.to_be_bytes());
    }
    rec.extend_from_slice(&0u16.to_be_bytes());
    rec.extend_from_slice(b"8BIM");
    rec.extend_from_slice(b"norm");
    rec.extend_from_slice(&[255, 0, 0, 0]);
    rec.extend_from_slice(&(extra.len() as u32).to_be_bytes());
    rec.extend_from_slice(&extra);

    let mut info = Vec::new();
    info.extend_from_slice(&1i16.to_be_bytes());
    info.extend_from_slice(&rec);
    while info.len() % 4 != 0 {
        info.push(0);
    }

    let mut out = header(1, 3, 1, 1, 3);
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    let section_len = 4 + info.len() + 4;
    out.extend_from_slice(&(section_len as u32).to_be_bytes());
    out.extend_from_slice(&(info.len() as u32).to_be_bytes());
    out.extend_from_slice(&info);
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&COMPRESSION_RAW.to_be_bytes());
    out.extend_from_slice(&[1, 2, 3]);
    out
}

fn has_key(bytes: &[u8], key: &[u8; 4]) -> bool {
    let mut needle = Vec::with_capacity(8);
    needle.extend_from_slice(b"8BIM");
    needle.extend_from_slice(key);
    bytes.windows(8).any(|w| w == needle)
}

#[test]
fn knko_clbl_infx_decode_and_leave_extra_blocks() {
    let psd = tagged_layer_psd(
        0,
        &[
            (b"knko", &[2, 0, 0, 0]),
            (b"clbl", &[0, 0, 0, 0]),
            (b"infx", &[0, 0, 0, 0]),
        ],
    );
    let layer = &read_psd(&psd).unwrap().layers[0];
    assert_eq!(layer.knockout, Knockout::Deep);
    assert!(!layer.blend_clipping);
    assert!(!layer.blend_interior);
    for key in [b"knko", b"clbl", b"infx"] {
        assert!(
            layer.extra_block(key).is_none(),
            "{key:?} must not stay in extra_blocks"
        );
    }

    let shallow = read_psd(&tagged_layer_psd(0, &[(b"knko", &[1])])).unwrap();
    assert_eq!(shallow.layers[0].knockout, Knockout::Shallow);
}

#[test]
fn advanced_blending_defaults_when_blocks_absent() {
    let layer = &read_psd(&tagged_layer_psd(0, &[])).unwrap().layers[0];
    assert_eq!(layer.knockout, Knockout::None);
    assert!(layer.blend_clipping);
    assert!(layer.blend_interior);
    assert!(layer.blend_if.is_none());
}

#[test]
fn empty_knko_payload_keeps_default_and_reads() {
    let layer = &read_psd(&tagged_layer_psd(0, &[(b"knko", &[])]))
        .unwrap()
        .layers[0];
    assert_eq!(layer.knockout, Knockout::None);
    assert!(layer.extra_block(b"knko").is_none());
}

#[test]
fn advanced_blending_non_defaults_round_trip_and_defaults_omit_keys() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("Adv", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    layer.knockout = Knockout::Deep;
    layer.blend_clipping = false;
    layer.blend_interior = false;
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let bytes = write_psd(&doc).unwrap();
    assert!(has_key(&bytes, b"knko"));
    assert!(has_key(&bytes, b"clbl"));
    assert!(has_key(&bytes, b"infx"));
    let back = read_psd(&bytes).unwrap();
    assert_eq!(back.layers[0].knockout, Knockout::Deep);
    assert!(!back.layers[0].blend_clipping);
    assert!(!back.layers[0].blend_interior);
    assert_eq!(back, doc);

    let mut defaults = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    defaults.layers = pictura_core::LayerStack::from_layers(vec![pixel(
        "Plain",
        rect(0, 0, 2, 2),
        3,
        BlendMode::Normal,
        255,
    )]);
    let plain = write_psd(&defaults).unwrap();
    for key in [b"knko", b"clbl", b"infx"] {
        assert!(!has_key(&plain, key), "{key:?} omitted at default");
    }
}

#[test]
fn non_empty_blending_ranges_decode_and_raw_round_trips() {
    // Composite source + dest (8 bytes) + one channel group (8 bytes).
    let mut body = Vec::new();
    for pair in [(0u16, 65535u16), (0, 65535), (10, 20), (30, 40)] {
        body.extend_from_slice(&pair.0.to_be_bytes());
        body.extend_from_slice(&pair.1.to_be_bytes());
    }

    let doc = read_psd(&ranges_layer_psd(&body)).unwrap();
    let layer = &doc.layers[0];
    assert_eq!(layer.blending_ranges, body, "raw field unchanged");
    let view = layer.blend_if.as_ref().expect("view decodes");
    assert_eq!(view.composite_source, (0u16, 65535u16));
    assert_eq!(view.composite_dest, (0u16, 65535u16));
    assert_eq!(view.channel_ranges, vec![((10u16, 20u16), (30, 40))]);

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.layers[0].blending_ranges, body);
    assert_eq!(back.layers[0].blend_if, layer.blend_if);
    assert_eq!(back, doc);
}

#[test]
fn empty_or_malformed_blending_ranges_leave_view_none() {
    let empty = read_psd(&ranges_layer_psd(&[])).unwrap();
    assert!(empty.layers[0].blending_ranges.is_empty());
    assert!(empty.layers[0].blend_if.is_none());

    // Composite-only body (8 bytes) is well-formed with no channel groups.
    let composite_only = vec![0u8; 8];
    let doc = read_psd(&ranges_layer_psd(&composite_only)).unwrap();
    let view = doc.layers[0].blend_if.as_ref().expect("composite decodes");
    assert_eq!(view.composite_source, (0, 0));
    assert!(view.channel_ranges.is_empty());
    assert_eq!(doc.layers[0].blending_ranges, composite_only);

    // Not a multiple of 8: view unset, document still reads, raw round-trips.
    let bad = vec![1u8; 12];
    let doc = read_psd(&ranges_layer_psd(&bad)).unwrap();
    assert!(doc.layers[0].blend_if.is_none());
    assert_eq!(doc.layers[0].blending_ranges, bad, "raw still round-trips");
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.layers[0].blending_ranges, bad);
    assert!(back.layers[0].blend_if.is_none());
}

#[test]
fn encode_blend_if_round_trips_through_parse() {
    let view = BlendIf {
        composite_source: (0u16, 64u16),
        composite_dest: (16u16, 32u16),
        channel_ranges: vec![((1u16, 2u16), (3, 4)), ((5, 6), (7, 8))],
    };
    let bytes = encode_blend_if(&view);
    assert_eq!(bytes.len(), 8 + 2 * 8);
    assert_eq!(
        crate::advanced_blending::parse_blend_if(&bytes),
        Some(view.clone())
    );

    let doc = read_psd(&ranges_layer_psd(&bytes)).unwrap();
    assert_eq!(doc.layers[0].blend_if.as_ref(), Some(&view));
}
