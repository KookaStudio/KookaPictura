mod clipping_invariants;

use super::*;

use crate::common::{
    COMPRESSION_RAW, COMPRESSION_RLE, COMPRESSION_ZIP, COMPRESSION_ZIP_PREDICTION,
};
use pictura_core::*;

fn header(version: u16, channels: u16, width: u32, height: u32, mode: u16) -> Vec<u8> {
    header_depth(version, channels, width, height, 8, mode)
}

fn header_depth(
    version: u16,
    channels: u16,
    width: u32,
    height: u32,
    depth: u16,
    mode: u16,
) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"8BPS");
    v.extend_from_slice(&version.to_be_bytes());
    v.extend_from_slice(&[0u8; 6]);
    v.extend_from_slice(&channels.to_be_bytes());
    v.extend_from_slice(&height.to_be_bytes());
    v.extend_from_slice(&width.to_be_bytes());
    v.extend_from_slice(&depth.to_be_bytes());
    v.extend_from_slice(&mode.to_be_bytes());
    v
}

fn psd_sections() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&0u32.to_be_bytes());
    v.extend_from_slice(&0u32.to_be_bytes());
    v.extend_from_slice(&0u32.to_be_bytes());
    v
}

/// zlib-frame `data` for a hand-built composite or channel payload.
fn zlib(data: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

/// Forward byte-wise delta per `row_len` scanline: inverse of the decoder's
/// `undo_prediction`.
fn predict(data: &[u8], row_len: usize) -> Vec<u8> {
    let mut out = data.to_vec();
    if row_len == 0 {
        return out;
    }
    for row_start in (0..out.len()).step_by(row_len) {
        let row_end = (row_start + row_len).min(out.len());
        for i in (row_start + 1)..row_end {
            out[i] = data[i].wrapping_sub(data[i - 1]);
        }
    }
    out
}

/// A 1x1 RGB PSD with one layer carrying a single `id=0` channel whose data is
/// `compression` followed by `encoded`.
fn one_layer_channel_psd(compression: u16, encoded: &[u8]) -> Vec<u8> {
    let channel_len = 2 + encoded.len();

    let mut extra = Vec::new();
    extra.extend_from_slice(&0u32.to_be_bytes()); // no mask
    extra.extend_from_slice(&0u32.to_be_bytes()); // blending ranges
    extra.extend_from_slice(&[1, b'L', 0, 0]); // pascal name "L"

    let mut rec = Vec::new();
    for v in [0i32, 0, 1, 1] {
        rec.extend_from_slice(&v.to_be_bytes());
    }
    rec.extend_from_slice(&1u16.to_be_bytes()); // one channel
    rec.extend_from_slice(&0i16.to_be_bytes()); // id 0
    rec.extend_from_slice(&(channel_len as u32).to_be_bytes());
    rec.extend_from_slice(b"8BIM");
    rec.extend_from_slice(b"norm");
    rec.push(255);
    rec.push(0);
    rec.push(0);
    rec.push(0);
    rec.extend_from_slice(&(extra.len() as u32).to_be_bytes());
    rec.extend_from_slice(&extra);

    let mut info = Vec::new();
    info.extend_from_slice(&1i16.to_be_bytes());
    info.extend_from_slice(&rec);
    info.extend_from_slice(&compression.to_be_bytes());
    info.extend_from_slice(encoded);
    while info.len() % 4 != 0 {
        info.push(0);
    }

    let mut out = header(1, 3, 1, 1, 3);
    out.extend_from_slice(&0u32.to_be_bytes()); // color mode data
    out.extend_from_slice(&0u32.to_be_bytes()); // image resources
    let section_len = 4 + info.len() + 4;
    out.extend_from_slice(&(section_len as u32).to_be_bytes());
    out.extend_from_slice(&(info.len() as u32).to_be_bytes());
    out.extend_from_slice(&info);
    out.extend_from_slice(&0u32.to_be_bytes()); // global layer mask
    out.extend_from_slice(&COMPRESSION_RAW.to_be_bytes());
    out.extend_from_slice(&[1, 2, 3]); // 1x1 RGB composite
    out
}

#[test]
fn parse_hand_constructed_raw_rgb() {
    let mut p = header(1, 3, 2, 2, 3);
    p.extend_from_slice(&psd_sections());
    p.extend_from_slice(&0u16.to_be_bytes());
    p.extend_from_slice(&[1, 2, 3, 4]); // R plane
    p.extend_from_slice(&[5, 6, 7, 8]); // G plane
    p.extend_from_slice(&[9, 10, 11, 12]); // B plane

    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.width, 2);
    assert_eq!(doc.height, 2);
    assert_eq!(doc.mode, ColorMode::Rgb);
    assert_eq!(doc.depth, BitDepth::Eight);
    assert_eq!(doc.composite.channels, 3);
    assert!(doc.layers.is_empty());
    assert_eq!(
        doc.composite.data,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
    );
}

#[test]
fn round_trip_rgb_and_grayscale() {
    for (mode, channels) in [(ColorMode::Rgb, 3u8), (ColorMode::Grayscale, 1u8)] {
        let mut doc = Document::new(3, 2, mode, BitDepth::Eight);
        for (i, b) in doc.composite.data.iter_mut().enumerate() {
            *b = (i * 7 + 3) as u8;
        }
        assert_eq!(doc.composite.channels, channels);
        let bytes = write_psd(&doc).unwrap();
        assert_eq!(read_psd(&bytes).unwrap(), doc);
    }
}

#[test]
fn extra_selection_channel_round_trips() {
    let mut doc = Document::new(4, 3, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 3) as u8;
    }
    doc.channels = vec![Channel {
        id: 0,
        data: (0..12).map(|i| 255 - (i * 5) as u8).collect(),
    }];

    let bytes = write_psd(&doc).unwrap();
    assert_eq!(u16::from_be_bytes(bytes[12..14].try_into().unwrap()), 4);

    let back = read_psd(&bytes).unwrap();
    assert_eq!(
        back.composite.channels, 3,
        "composite keeps only color planes"
    );
    assert_eq!(back.channels, doc.channels);
    assert_eq!(back, doc);
}

#[test]
fn multiple_extra_channels_round_trip() {
    let mut doc = Document::new(2, 2, ColorMode::Grayscale, BitDepth::Eight);
    doc.composite.data.copy_from_slice(&[1, 2, 3, 4]);
    doc.channels = vec![
        Channel {
            id: 0,
            data: vec![10, 20, 30, 40].into(),
        },
        Channel {
            id: 1,
            data: vec![50, 60, 70, 80].into(),
        },
    ];

    let bytes = write_psd(&doc).unwrap();
    assert_eq!(u16::from_be_bytes(bytes[12..14].try_into().unwrap()), 3);

    let back = read_psd(&bytes).unwrap();
    assert_eq!(back.composite.channels, 1);
    assert_eq!(back, doc);
}

#[test]
fn extra_channel_length_mismatch_is_rejected() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.channels = vec![Channel {
        id: 0,
        data: vec![0; 3].into(),
    }];
    assert!(matches!(write_psd(&doc), Err(PsdError::Invalid(_))));
}

#[test]
fn parse_rle_psd() {
    let mut p = header(1, 1, 4, 2, 1);
    p.extend_from_slice(&psd_sections());
    p.extend_from_slice(&1u16.to_be_bytes()); // RLE
    p.extend_from_slice(&5u16.to_be_bytes()); // row 0 packed length
    p.extend_from_slice(&2u16.to_be_bytes()); // row 1 packed length
    p.push(3);
    p.extend_from_slice(&[0, 1, 2, 3]); // literal run of 4
    p.push(253);
    p.push(5); // repeat 5 four times

    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.composite.data, vec![0, 1, 2, 3, 5, 5, 5, 5]);
}

#[test]
fn parse_rle_psb_uses_four_byte_counts() {
    let mut p = header(2, 1, 4, 1, 1);
    p.extend_from_slice(&0u32.to_be_bytes()); // color mode
    p.extend_from_slice(&0u32.to_be_bytes()); // image resources
    p.extend_from_slice(&0u64.to_be_bytes()); // PSB layer/mask is 8 bytes
    p.extend_from_slice(&1u16.to_be_bytes()); // RLE
    p.extend_from_slice(&5u32.to_be_bytes()); // 4-byte scanline count
    p.push(3);
    p.extend_from_slice(&[9, 8, 7, 6]);

    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.composite.data, vec![9, 8, 7, 6]);
}

#[test]
fn bad_signature_is_error_not_panic() {
    let mut p = header(1, 3, 1, 1, 3);
    p[0] = b'X';
    assert!(matches!(read_psd(&p), Err(PsdError::BadSignature(_))));
}

#[test]
fn truncated_is_error_not_panic() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = (0..12).collect();
    let bytes = write_psd(&doc).unwrap();
    assert!(matches!(
        read_psd(&bytes[..bytes.len() - 1]),
        Err(PsdError::Truncated)
    ));
    assert!(matches!(read_psd(b"8BPS"), Err(PsdError::Truncated)));
}

#[test]
fn lcg_round_trip_property() {
    let mut state: u64 = 0x1234_5678_9abc_def0;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as u32
    };

    for _ in 0..40 {
        let mode = if next() % 2 == 0 {
            ColorMode::Rgb
        } else {
            ColorMode::Grayscale
        };
        let channels = if mode == ColorMode::Rgb { 3 } else { 1 };
        let width = 1 + next() % 17;
        let height = 1 + next() % 17;

        let mut doc = Document::new(width, height, mode, BitDepth::Eight);
        for b in doc.composite.data.iter_mut() {
            *b = next() as u8;
        }
        let bytes = write_psd(&doc).unwrap();
        let back = read_psd(&bytes).unwrap();
        assert_eq!(back, doc, "mismatch for {width}x{height} ch={channels}");
    }
}

// -- M1-B: layers ------------------------------------------------------

fn rect(top: i32, left: i32, bottom: i32, right: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

fn pixel(name: &str, r: PsdRect, color_channels: u8, blend: BlendMode, opacity: u8) -> Layer {
    let width = r.width().max(0) as usize;
    let height = r.height().max(0) as usize;
    let channels = (0..color_channels)
        .map(|c| Channel {
            id: c as i16,
            data: vec![c * 40 + 17; width * height].into(),
        })
        .chain(std::iter::once(Channel {
            id: -1,
            data: vec![255; width * height].into(),
        }))
        .collect();
    Layer {
        name: name.to_string(),
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
        channels,
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

#[test]
fn round_trip_layers_group_and_mask() {
    let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }

    let red = pixel("Red", rect(0, 0, 4, 4), 3, BlendMode::Multiply, 200);
    let green = pixel("Green", rect(4, 4, 8, 8), 3, BlendMode::Screen, 255);
    let blue = pixel("Blue", rect(8, 8, 12, 12), 3, BlendMode::Normal, 128);
    let group = Layer {
        name: "Group A".to_string(),
        rect: rect(0, 0, 0, 0),
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children: pictura_core::LayerStack::from_layers(vec![green, blue]),
        is_group: true,
        background: false,
        ..Default::default()
    };

    let mut masked = pixel("Masked", rect(2, 2, 6, 6), 3, BlendMode::Overlay, 255);
    masked.mask = Some(LayerMask {
        rect: rect(2, 2, 6, 6),
        default_color: 0,
        disabled: true,
        flags: 0x02,
        data: Some(vec![7u8; 16].into()),
        ..Default::default()
    });

    doc.layers = pictura_core::LayerStack::from_layers(vec![red, group, masked]);

    let bytes = write_psd(&doc).unwrap();
    let back = read_psd(&bytes).unwrap();

    assert_eq!(back.layers.len(), 3);
    assert_eq!(back.layers[0].name, "Red");
    assert_eq!(back.layers[0].blend, BlendMode::Multiply);
    assert_eq!(back.layers[0].opacity, 200);
    assert_eq!(back.layers[0].rect, rect(0, 0, 4, 4));
    assert!(back.layers[1].is_group());
    assert_eq!(back.layers[1].name, "Group A");
    assert_eq!(back.layers[1].children.len(), 2);
    assert_eq!(back.layers[1].children[0].name, "Green");
    assert_eq!(back.layers[1].children[1].name, "Blue");
    assert_eq!(back.layers[1].children[0].rect, rect(4, 4, 8, 8));
    let mask = back.layers[2].mask.as_ref().expect("mask round-trips");
    assert_eq!(mask.rect, rect(2, 2, 6, 6));
    assert!(mask.disabled);
    assert_eq!(mask.data.as_deref(), Some(&[7u8; 16][..]));

    // The strongest check: the whole document is equal.
    assert_eq!(back, doc);
}

#[test]
fn pass_through_group_round_trips() {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    let child = pixel("Child", rect(0, 0, 4, 4), 3, BlendMode::Multiply, 255);
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        name: "Pass Group".to_string(),
        rect: rect(0, 0, 0, 0),
        blend: BlendMode::PassThrough,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children: pictura_core::LayerStack::from_layers(vec![child]),
        is_group: true,
        background: false,
        ..Default::default()
    }]);

    let bytes = write_psd(&doc).unwrap();
    let back = read_psd(&bytes).unwrap();
    assert!(back.layers[0].is_group());
    assert_eq!(back.layers[0].blend, BlendMode::PassThrough);
    assert_eq!(back, doc);
}

#[test]
fn gray_layer_round_trips() {
    let mut doc = Document::new(8, 8, ColorMode::Grayscale, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![pixel(
        "Gray",
        rect(0, 0, 8, 8),
        1,
        BlendMode::Normal,
        255,
    )]);
    let bytes = write_psd(&doc).unwrap();
    assert_eq!(read_psd(&bytes).unwrap(), doc);
}

#[test]
fn adjustment_layers_round_trip_key_and_bytes() {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }
    let base = pixel("Base", rect(0, 0, 8, 8), 3, BlendMode::Normal, 255);

    // Empty (Invert), descriptor-ish, and even payloads. An additional-layer
    // block declares an even length with the pad inside, so payloads here are
    // even; the odd-payload normalization is covered separately.
    let cases: &[([u8; 4], Vec<u8>)] = &[
        (*b"nvrt", Vec::new()),
        (*b"post", vec![0, 4, 0, 0]),
        (*b"thrs", vec![0, 128, 0, 0]),
        (*b"brit", vec![0, 10, 0, 20, 0, 0, 0, 0]),
        (*b"hue2", vec![0; 16]),
        (*b"blnc", vec![0; 20]),
        (*b"mixr", vec![0; 44]),
        (*b"curv", vec![1, 2, 3, 4]),
        (*b"selc", vec![0u8; 84]),
        (*b"clrL", vec![0u8; 12]),
    ];

    let mut layers = vec![base];
    for (i, (key, data)) in cases.iter().enumerate() {
        layers.push(Layer {
            name: format!("adj{i}"),
            rect: rect(0, 0, 0, 0),
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            color: ColorLabel::None,
            clipping: false,
            visible: true,
            mask: None,
            adjustment: Some(AdjustmentData {
                key: *key,
                data: data.clone(),
            }),
            channels: Vec::new(),
            children: pictura_core::LayerStack::default(),
            is_group: false,
            background: false,
            ..Default::default()
        });
    }
    doc.layers = pictura_core::LayerStack::from_layers(layers);

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc);
    for (i, (key, data)) in cases.iter().enumerate() {
        let adj = back.layers[i + 1]
            .adjustment
            .as_ref()
            .expect("adjustment round-trips");
        assert_eq!(&adj.key, key);
        assert_eq!(&adj.data, data);
    }
}

#[test]
fn solid_fill_layer_round_trips_its_payload() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }
    let fill = Layer {
        name: "Color Fill 1".into(),
        rect: rect(0, 0, 4, 4),
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: Some(AdjustmentData {
            key: *b"SoCo",
            data: vec![10, 20, 30, 255],
        }),
        channels: Vec::new(),
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    };
    doc.layers = pictura_core::LayerStack::from_layers(vec![fill]);

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc);
    let adj = back.layers[0]
        .adjustment
        .as_ref()
        .expect("the SoCo block round-trips");
    assert_eq!(&adj.key, b"SoCo");
    assert_eq!(adj.data, [10, 20, 30, 255]);
}

#[test]
fn descriptor_shaped_soco_is_preserved_verbatim() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 3 + 1) as u8;
    }
    // A real `'Clr '` descriptor is longer than the 4-byte subset; the codec
    // must carry it unchanged rather than interpreting it.
    let descriptor = vec![0u8, 0, 0, 0, 0, 1, 8, 66, 73, 77, 67, 108, 114, 32];
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        name: "Descriptor Fill".into(),
        rect: rect(0, 0, 0, 0),
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: Some(AdjustmentData {
            key: *b"SoCo",
            data: descriptor.clone(),
        }),
        channels: Vec::new(),
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }]);

    let bytes = write_psd(&doc).unwrap();
    let back = read_psd(&bytes).unwrap();
    assert_eq!(back, doc);
    assert_eq!(back.layers[0].adjustment.as_ref().unwrap().data, descriptor);
}

#[test]
fn malformed_layer_section_is_error_not_panic() {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![pixel(
        "Only",
        rect(0, 0, 8, 8),
        3,
        BlendMode::Normal,
        255,
    )]);
    let good = write_psd(&doc).unwrap();

    // Truncated inside the layer/mask section.
    assert!(read_psd(&good[..60]).is_err());

    // Bogus layer count: claims 100 records but the section holds one.
    let mut bogus = good.clone();
    bogus[42..44].copy_from_slice(&100i16.to_be_bytes());
    assert!(read_psd(&bogus).is_err());

    // Bogus channel data length: first channel info length at offset 64
    // (header 26 + section lengths 12 + count 2 + rect 16 + nch 2 + id 2).
    let mut bad_len = good.clone();
    bad_len[64..68].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(read_psd(&bad_len).is_err());
}

#[test]
fn invalid_zip_layer_payload_errors() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![pixel(
        "Only",
        rect(0, 0, 4, 4),
        3,
        BlendMode::Normal,
        255,
    )]);
    let mut bytes = write_psd(&doc).unwrap();
    // Layer channel data follows the first (and only) record's extra data.
    let extra_len = u32::from_be_bytes(bytes[98..102].try_into().unwrap()) as usize;
    let channel_data = 102 + extra_len;
    bytes[channel_data + 1] = 2; // compression 2 = ZIP, but the payload is raw
    assert!(matches!(read_psd(&bytes), Err(PsdError::Unsupported(_))));
}

#[test]
fn layer_with_too_many_channels_is_rejected() {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("TooMany", rect(0, 0, 1, 1), 3, BlendMode::Normal, 255);
    layer.channels = (0..=crate::common::MAX_CHANNELS as i16)
        .map(|id| Channel {
            id,
            data: vec![0; 1].into(),
        })
        .collect();
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    assert!(matches!(
        write_psd(&doc),
        Err(PsdError::Invalid(msg)) if msg.contains("layer channel count")
    ));
}

// -- M36: lspf / lclr / iOpa -------------------------------------------

/// The fixed default document used for the serialization golden.
fn default_document() -> Document {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 7 + 1) as u8;
    }
    let mut masked = pixel("Masked", rect(0, 0, 2, 2), 3, BlendMode::Normal, 200);
    masked.mask = Some(LayerMask {
        rect: rect(0, 0, 2, 2),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(vec![9u8; 4].into()),
        ..Default::default()
    });
    let adj = Layer {
        name: "Invert".to_string(),
        rect: rect(0, 0, 0, 0),
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: Some(AdjustmentData {
            key: *b"nvrt",
            data: Vec::new(),
        }),
        channels: Vec::new(),
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    };
    let group = Layer {
        name: "Group".to_string(),
        rect: rect(0, 0, 0, 0),
        blend: BlendMode::PassThrough,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children: pictura_core::LayerStack::from_layers(vec![pixel(
            "Inner",
            rect(1, 1, 3, 3),
            3,
            BlendMode::Multiply,
            128,
        )]),
        is_group: true,
        background: false,
        ..Default::default()
    };
    doc.layers = pictura_core::LayerStack::from_layers(vec![masked, adj, group]);
    doc
}

#[test]
fn default_document_matches_rle_golden() {
    let bytes = write_psd(&default_document()).unwrap();
    let golden = include_bytes!("../tests/fixtures/default_before.psd");
    assert_eq!(
        bytes.as_slice(),
        golden.as_slice(),
        "default documents must serialize byte-identically to the PackBits-RLE baseline"
    );
}

// -- PackBits RLE write -------------------------------------------------

#[test]
fn encode_rle_round_trips() {
    let cases: &[(usize, usize, &[u8])] = &[
        (5, 3, &[9; 15]),
        (4, 1, &[1, 2, 3, 4]),
        (1, 1, &[7]),
        (0, 0, &[]),
    ];
    for &(w, h, plane) in cases {
        let payload = crate::write::encode_scanlines(&[plane], w, h, false).unwrap();
        let decoded = crate::read::decode_rle_channel(&payload, w, h, false).unwrap();
        assert_eq!(decoded, plane, "w={w} h={h}");
    }

    // Widths past the 128-byte packet boundary and a run longer than one
    // repeat packet can hold.
    for plane in [
        &(0..258).map(|i| (i % 7) as u8).collect::<Vec<u8>>(),
        &vec![5u8; 300],
    ] {
        let payload = crate::write::encode_scanlines(&[plane], plane.len(), 1, false).unwrap();
        let decoded = crate::read::decode_rle_channel(&payload, plane.len(), 1, false).unwrap();
        assert_eq!(&decoded, plane);
    }
}

#[test]
fn written_composite_declares_rle() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 11 + 3) as u8;
    }
    let bytes = write_psd(&doc).unwrap();
    // Header (26) + empty color-mode-data (4) + empty image-resources (4) +
    // empty layer/mask section (4) precede the image-data compression word.
    let compression = u16::from_be_bytes([bytes[38], bytes[39]]);
    assert_eq!(
        compression, COMPRESSION_RLE,
        "composite declares PackBits RLE"
    );
    assert_eq!(read_psd(&bytes).unwrap(), doc, "RLE composite round-trips");
}

#[test]
fn rle_is_smaller_for_compressible_image() {
    let mut doc = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data.fill(200);
    let bytes = write_psd(&doc).unwrap();
    // A raw write would carry 3*64*64 = 12 288 composite bytes; the whole RLE
    // file must be smaller than just those pixels.
    assert!(
        bytes.len() < doc.composite.data.len(),
        "uniform composite should RLE-compress: {} >= {}",
        bytes.len(),
        doc.composite.data.len()
    );
}

#[test]
fn verbatim_channels_stay_unchanged() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("RawHold", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    let stream = vec![0u8, 3, 1, 2, 3, 0xFF, 0xFE];
    layer.raw_channels = vec![RawChannel {
        id: -3,
        data: stream.clone(),
    }];
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let bytes = write_psd(&doc).unwrap();
    let back = read_psd(&bytes).unwrap();
    assert_eq!(back.layers[0].raw_channels.len(), 1);
    assert_eq!(back.layers[0].raw_channels[0].data, stream);
    assert_eq!(back, doc, "document with a preserved channel round-trips");
}

#[test]
fn layer_attributes_round_trip() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 5) as u8;
    }
    let mut layer = pixel("Attrs", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    layer.fill = 128;
    layer.color = ColorLabel::Violet;
    layer.lock = LockFlags::default()
        .with(LockFlags::TRANSPARENCY, true)
        .with(LockFlags::POSITION, true);
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let bytes = write_psd(&doc).unwrap();
    let back = read_psd(&bytes).unwrap();
    assert_eq!(back, doc, "whole document round-trips");
    assert_eq!(back.layers[0].fill, 128);
    assert_eq!(back.layers[0].color, ColorLabel::Violet);
    assert_eq!(back.layers[0].lock.bits(), 0x05);
}

#[test]
fn nesting_lock_bit_round_trips_through_lspf() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("Nested", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    layer.lock = LockFlags::default().with(LockFlags::NESTING, true);
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let bytes = write_psd(&doc).unwrap();
    let back = read_psd(&bytes).unwrap();
    assert!(back.layers[0].lock.contains(LockFlags::NESTING));
    assert_eq!(back.layers[0].lock.bits(), 0x08);
    assert_eq!(back, doc, "whole document round-trips");
}

#[test]
fn background_flag_round_trips_and_is_derived_on_read() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut background = pixel("Background", rect(0, 0, 4, 4), 3, BlendMode::Normal, 255);
    background.background = true;
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        background,
        pixel("top", rect(0, 0, 4, 4), 3, BlendMode::Normal, 255),
    ]);

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert!(back.layers[0].background, "the bottom flag round-trips");
    assert_eq!(back, doc);

    // A flagged layer under any name is written as `"Background"`, and the
    // reader derives the flag from the bottom position and that name.
    let mut renamed = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("Base", rect(0, 0, 4, 4), 3, BlendMode::Normal, 255);
    layer.background = true;
    renamed.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    let back = read_psd(&write_psd(&renamed).unwrap()).unwrap();
    assert_eq!(back.layers[0].name, "Background");
    assert!(back.layers[0].background);
}

/// Assemble a 1x1 RGB PSD with one channel-less layer whose record carries
/// `flags` and the given hand-built tagged blocks.
fn tagged_layer_psd(flags: u8, tags: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    tagged_layer_psd_with_signature(b"8BIM", flags, tags)
}

/// As [`tagged_layer_psd`], but with an explicit tagged-block signature so a
/// test can exercise `8B64` or a bogus one.
fn tagged_layer_psd_with_signature(
    signature: &[u8; 4],
    flags: u8,
    tags: &[(&[u8; 4], &[u8])],
) -> Vec<u8> {
    let mut extra = Vec::new();
    extra.extend_from_slice(&0u32.to_be_bytes()); // mask data length
    extra.extend_from_slice(&0u32.to_be_bytes()); // blending ranges
    extra.extend_from_slice(&[1, b'L', 0, 0]); // pascal name "L"
    for (key, data) in tags {
        extra.extend_from_slice(signature);
        extra.extend_from_slice(*key);
        extra.extend_from_slice(&(data.len() as u32).to_be_bytes());
        extra.extend_from_slice(data);
        if data.len() % 2 == 1 {
            extra.push(0);
        }
    }
    if extra.len() % 2 == 1 {
        extra.push(0);
    }

    let mut rec = Vec::new();
    for v in [0i32, 0, 1, 1] {
        rec.extend_from_slice(&v.to_be_bytes());
    }
    rec.extend_from_slice(&0u16.to_be_bytes()); // no channels
    rec.extend_from_slice(b"8BIM");
    rec.extend_from_slice(b"norm");
    rec.push(255);
    rec.push(0);
    rec.push(flags);
    rec.push(0);
    rec.extend_from_slice(&(extra.len() as u32).to_be_bytes());
    rec.extend_from_slice(&extra);

    let mut info = Vec::new();
    info.extend_from_slice(&1i16.to_be_bytes());
    info.extend_from_slice(&rec);
    while info.len() % 4 != 0 {
        info.push(0);
    }

    let mut out = header(1, 3, 1, 1, 3);
    out.extend_from_slice(&0u32.to_be_bytes()); // color mode data
    out.extend_from_slice(&0u32.to_be_bytes()); // image resources
    let section_len = 4 + info.len() + 4;
    out.extend_from_slice(&(section_len as u32).to_be_bytes());
    out.extend_from_slice(&(info.len() as u32).to_be_bytes());
    out.extend_from_slice(&info);
    out.extend_from_slice(&0u32.to_be_bytes()); // global layer mask
    out.extend_from_slice(&COMPRESSION_RAW.to_be_bytes());
    out.extend_from_slice(&[1, 2, 3]); // 1x1 RGB composite
    out
}

#[test]
fn reads_hand_built_attribute_tags_and_folds_legacy_flag() {
    let lspf = 2u32.to_be_bytes(); // lock image pixels
    let mut lclr = [0u8; 8];
    lclr[1] = 2; // Orange
    let iopa = [128u8, 0, 0, 0]; // 4-byte payload: reader takes byte 0

    let psd = tagged_layer_psd(
        0x01, // legacy transparency-protected bit
        &[(b"lspf", &lspf), (b"lclr", &lclr), (b"iOpa", &iopa)],
    );
    let doc = read_psd(&psd).unwrap();
    let layer = &doc.layers[0];
    assert_eq!(layer.fill, 128);
    assert_eq!(layer.color, ColorLabel::Orange);
    assert!(layer.lock.contains(LockFlags::PIXELS));
    assert!(
        layer.lock.contains(LockFlags::TRANSPARENCY),
        "legacy bit folds in"
    );
    assert!(!layer.lock.contains(LockFlags::POSITION));
}

#[test]
fn an_8b64_tagged_block_is_accepted_and_preserved() {
    let payload = [1u8, 2, 3, 4];
    let psd = tagged_layer_psd_with_signature(b"8B64", 0, &[(b"zzzz", &payload)]);
    let doc = read_psd(&psd).expect("an 8B64 tagged block reads");
    assert_eq!(
        doc.layers[0].extra_block(b"zzzz").unwrap().data.as_slice(),
        &payload
    );

    // The key and payload survive a round trip; the signature normalizes to 8BIM.
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(
        back.layers[0].extra_block(b"zzzz").unwrap().data.as_slice(),
        &payload
    );
}

#[test]
fn a_bogus_tagged_block_signature_errors() {
    let psd = tagged_layer_psd_with_signature(b"zzzz", 0, &[(b"zzzz", b"x")]);
    assert!(read_psd(&psd).is_err());
}

#[test]
fn tysh_forces_type_locks_and_round_trips() {
    let tysh = [0xAB, 0xCD];
    let lspf_clear = 0u32.to_be_bytes();
    let psd = tagged_layer_psd(0, &[(b"lspf", &lspf_clear), (b"TySh", &tysh)]);
    let doc = read_psd(&psd).unwrap();
    let layer = &doc.layers[0];
    assert!(layer.is_type());
    assert_eq!(layer.lock.bits(), 0x03, "TRANSPARENCY|PIXELS forced");
    assert_eq!(layer.extra_block(b"TySh").unwrap().data, tysh);

    let plain = read_psd(&tagged_layer_psd(0, &[(b"lspf", &lspf_clear)])).unwrap();
    assert!(!plain.layers[0].is_type());
    assert_eq!(plain.layers[0].lock.bits(), 0, "no TySh keeps stored locks");

    // POSITION stays set while TySh forces the two CS6 type locks.
    let lspf_pos = 4u32.to_be_bytes();
    let with_pos = read_psd(&tagged_layer_psd(
        0,
        &[(b"lspf", &lspf_pos), (b"TySh", &tysh)],
    ))
    .unwrap();
    assert_eq!(with_pos.layers[0].lock.bits(), 0x07);

    // open→save keeps TySh bytes; source locks match the forced defaults so
    // the full document still round-trips equal.
    let mut out = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut type_layer = pixel("Type", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    type_layer.lock = LockFlags::default()
        .with(LockFlags::TRANSPARENCY, true)
        .with(LockFlags::PIXELS, true);
    type_layer.extra_blocks = vec![LayerBlock {
        key: *b"TySh",
        data: vec![9, 8, 7, 6],
    }];
    out.layers = pictura_core::LayerStack::from_layers(vec![type_layer]);
    let back = read_psd(&write_psd(&out).unwrap()).unwrap();
    assert_eq!(back, out);
    assert!(back.layers[0].is_type());
    assert_eq!(
        back.layers[0].extra_block(b"TySh").unwrap().data,
        vec![9, 8, 7, 6]
    );
}

fn synthetic_tysh() -> Vec<u8> {
    let text_desc = write_descriptor(&DescValue::Object {
        name: String::new(),
        class_id: b"TxtX".to_vec(),
        items: vec![(b"Txt ".to_vec(), DescValue::Text("Hi\rThere".into()))],
    });
    let warp_desc = write_descriptor(&DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: Vec::new(),
    });
    encode_type_tool(&TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, 10.0, 20.0],
        text: "Hi\rThere".into(),
        bounds: [0, 0, 100, 50],
        text_desc,
        warp_desc,
        fonts: Vec::new(),
        style: None,
        vertical: false,
        box_bounds: None,
    })
}

#[test]
fn type_tool_open_save_preserves_bytes() {
    let tysh = synthetic_tysh();
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut type_layer = pixel("Type", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    type_layer.lock = LockFlags::default()
        .with(LockFlags::TRANSPARENCY, true)
        .with(LockFlags::PIXELS, true);
    type_layer.extra_blocks = vec![LayerBlock {
        key: *b"TySh",
        data: tysh.clone(),
    }];
    doc.layers = pictura_core::LayerStack::from_layers(vec![type_layer]);

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.layers[0].extra_block(b"TySh").unwrap().data, tysh);
    let tool = back.layers[0].type_tool.as_ref().expect("view decodes");
    assert_eq!(tool.transform, [1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
    assert_eq!(tool.text, "Hi\rThere");
    assert_eq!(tool.bounds, [0, 0, 100, 50]);
}

#[test]
fn malformed_tysh_leaves_view_none() {
    let bad_version = [0u8, 2];
    let doc = read_psd(&tagged_layer_psd(0, &[(b"TySh", &bad_version)])).unwrap();
    assert!(doc.layers[0].is_type(), "kind detection is presence-only");
    assert!(doc.layers[0].type_tool.is_none(), "version != 1");

    let good = synthetic_tysh();
    let truncated = &good[..20];
    let doc = read_psd(&tagged_layer_psd(0, &[(b"TySh", truncated)])).unwrap();
    assert!(doc.layers[0].is_type());
    assert!(doc.layers[0].type_tool.is_none(), "truncated descriptor");
}

// -- ZIP compression and tolerant open ----------------------------------

#[test]
fn zip_composite_round_trips() {
    let planes = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let mut p = header(1, 3, 2, 2, 3);
    p.extend_from_slice(&psd_sections());
    p.extend_from_slice(&COMPRESSION_ZIP.to_be_bytes());
    p.extend_from_slice(&zlib(&planes));

    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.composite.channels, 3);
    assert_eq!(doc.composite.data, planes.to_vec());
    assert!(doc.merged_composite_present);
}

#[test]
fn zip_prediction_composite_round_trips() {
    let planes = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let encoded = predict(&planes, 2);
    let mut p = header(1, 3, 2, 2, 3);
    p.extend_from_slice(&psd_sections());
    p.extend_from_slice(&COMPRESSION_ZIP_PREDICTION.to_be_bytes());
    p.extend_from_slice(&zlib(&encoded));

    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.composite.data, planes.to_vec());
}

#[test]
fn zip_layer_channel_decodes() {
    let expected = vec![0xAB];
    let psd = one_layer_channel_psd(COMPRESSION_ZIP, &zlib(&expected));
    let doc = read_psd(&psd).unwrap();
    assert_eq!(doc.layers[0].channels.len(), 1);
    assert_eq!(doc.layers[0].channels[0].data, expected);
}

#[test]
fn zip_prediction_layer_channel_decodes() {
    let expected = vec![0x11];
    let psd = one_layer_channel_psd(COMPRESSION_ZIP_PREDICTION, &zlib(&expected));
    let doc = read_psd(&psd).unwrap();
    assert_eq!(doc.layers[0].channels[0].data, expected);
}

// -- Compression preservation on write ----------------------------------

fn patterned_document() -> Document {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 11 + 3) as u8;
    }
    doc
}

#[test]
fn zip_composite_declares_zip_and_round_trips() {
    let mut doc = patterned_document();
    doc.composite_compression = Compression::Zip;
    let bytes = write_psd(&doc).unwrap();
    let compression = u16::from_be_bytes([bytes[38], bytes[39]]);
    assert_eq!(compression, COMPRESSION_ZIP, "composite declares ZIP");
    assert_eq!(read_psd(&bytes).unwrap(), doc, "ZIP composite round-trips");
}

#[test]
fn zip_prediction_composite_declares_three_and_round_trips() {
    let mut doc = patterned_document();
    doc.composite_compression = Compression::ZipPrediction;
    let bytes = write_psd(&doc).unwrap();
    let compression = u16::from_be_bytes([bytes[38], bytes[39]]);
    assert_eq!(compression, COMPRESSION_ZIP_PREDICTION);
    assert_eq!(read_psd(&bytes).unwrap(), doc, "prediction round-trips");
}

#[test]
fn raw_composite_declares_raw_and_round_trips() {
    let mut doc = patterned_document();
    doc.composite_compression = Compression::Raw;
    let bytes = write_psd(&doc).unwrap();
    let compression = u16::from_be_bytes([bytes[38], bytes[39]]);
    assert_eq!(compression, COMPRESSION_RAW);
    assert_eq!(read_psd(&bytes).unwrap(), doc, "raw composite round-trips");
}

#[test]
fn zip_layer_channel_and_mask_round_trip() {
    let mut doc = patterned_document();
    doc.layer_compression = Compression::Zip;
    let mut layer = pixel("Zip", rect(0, 0, 4, 4), 3, BlendMode::Normal, 255);
    layer.mask = Some(LayerMask {
        rect: rect(0, 0, 4, 4),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some((0..16).map(|i| (i * 5) as u8).collect()),
        ..Default::default()
    });
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let bytes = write_psd(&doc).unwrap();
    let back = read_psd(&bytes).unwrap();
    assert_eq!(back, doc, "ZIP layer channel and mask round-trip");
}

#[test]
fn zip_write_is_deterministic() {
    let mut doc = patterned_document();
    doc.composite_compression = Compression::Zip;
    doc.layer_compression = Compression::Zip;
    doc.layers = pictura_core::LayerStack::from_layers(vec![pixel(
        "Zip",
        rect(0, 0, 4, 4),
        3,
        BlendMode::Normal,
        255,
    )]);
    assert_eq!(write_psd(&doc).unwrap(), write_psd(&doc).unwrap());
}

#[test]
fn read_records_compression_kinds() {
    // Composite ZIP with no layers: the layer kind keeps the RLE default.
    let mut p = header(1, 3, 2, 2, 3);
    p.extend_from_slice(&psd_sections());
    p.extend_from_slice(&COMPRESSION_ZIP.to_be_bytes());
    p.extend_from_slice(&zlib(&[1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]));
    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.composite_compression, Compression::Zip);
    assert_eq!(doc.layer_compression, Compression::Rle);

    // One ZIP layer channel over a raw composite.
    let layer_psd = one_layer_channel_psd(COMPRESSION_ZIP, &zlib(&[0xAB]));
    let doc = read_psd(&layer_psd).unwrap();
    assert_eq!(doc.composite_compression, Compression::Raw);
    assert_eq!(doc.layer_compression, Compression::Zip);

    let layer_psd = one_layer_channel_psd(COMPRESSION_ZIP_PREDICTION, &zlib(&[0x11]));
    let doc = read_psd(&layer_psd).unwrap();
    assert_eq!(doc.layer_compression, Compression::ZipPrediction);
}

#[test]
fn group_fixture_records_layer_compression_from_surviving_channels() {
    // The leading 0x0 bounding-section-divider record declares RAW channels that
    // `build_tree` discards; only the surviving leaf channels may set the kind.
    let doc = read_psd(include_bytes!("../tests/fixtures/group.psd")).unwrap();
    assert_eq!(
        doc.layer_compression,
        Compression::Rle,
        "discarded divider/folder channels must not set the kind"
    );
    assert_eq!(read_psd(&write_psd(&doc).unwrap()).unwrap(), doc);
}

#[test]
fn empty_group_document_round_trips_with_default_layer_compression() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        name: "Group".to_string(),
        rect: rect(0, 0, 0, 0),
        blend: BlendMode::PassThrough,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children: pictura_core::LayerStack::default(),
        is_group: true,
        background: false,
        ..Default::default()
    }]);

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.layer_compression, Compression::Rle);
    assert_eq!(back, doc);
}

#[test]
fn no_composite_document_writes_no_image_data_and_round_trips() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![pixel(
        "Only",
        rect(0, 0, 4, 4),
        3,
        BlendMode::Normal,
        255,
    )]);
    doc.merged_composite_present = false;

    let bytes = write_psd(&doc).unwrap();
    // No image-data section: the file ends at the layer/mask section.
    let section_len = u32::from_be_bytes(bytes[34..38].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), 38 + section_len, "no image-data section");

    let back = read_psd(&bytes).unwrap();
    assert!(!back.merged_composite_present);
    assert_eq!(back, doc);
}

#[test]
fn zip_prediction_is_per_row() {
    // A per-plane inverse would add row 0's last byte into row 1's first byte,
    // turning 30 into 50; the per-row inverse leaves it at 30.
    let planes = [10u8, 20, 30, 40];
    let encoded = predict(&planes, 2);
    let mut p = header(1, 1, 2, 2, 1);
    p.extend_from_slice(&psd_sections());
    p.extend_from_slice(&COMPRESSION_ZIP_PREDICTION.to_be_bytes());
    p.extend_from_slice(&zlib(&encoded));

    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.composite.data, planes.to_vec());
}

#[test]
fn unknown_blend_key_reads_as_normal() {
    let mut psd = tagged_layer_psd(0, &[]);
    // Blend key follows the record's "8BIM" at offset 62.
    psd[66..70].copy_from_slice(b"zzzz");

    let doc = read_psd(&psd).unwrap();
    assert_eq!(doc.layers[0].blend, BlendMode::Normal);
}

#[test]
fn absent_composite_yields_zero_composite() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![pixel(
        "Only",
        rect(0, 0, 4, 4),
        3,
        BlendMode::Normal,
        255,
    )]);
    let bytes = write_psd(&doc).unwrap();
    let section_len = u32::from_be_bytes(bytes[34..38].try_into().unwrap()) as usize;

    let back = read_psd(&bytes[..38 + section_len]).unwrap();
    assert_eq!(back.layers.len(), 1);
    assert_eq!(back.composite.channels, 3);
    assert_eq!(back.composite.data, vec![0u8; 4 * 4 * 3]);
    assert!(!back.merged_composite_present);

    // A bare header with no layer section still errors as truncated.
    assert!(read_psd(&header(1, 3, 1, 1, 3)).is_err());
}

#[test]
fn bad_zip_payload_errors() {
    let mut p = header(1, 3, 1, 1, 3);
    p.extend_from_slice(&psd_sections());
    p.extend_from_slice(&COMPRESSION_ZIP.to_be_bytes());
    p.extend_from_slice(&[0xff; 8]);

    assert!(matches!(
        read_psd(&p),
        Err(PsdError::Unsupported(_)) | Err(PsdError::Invalid(_))
    ));
}

// -- Opaque preservation ------------------------------------------------

#[test]
fn opaque_document_sections_round_trip() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = (0..12).collect();
    doc.color_mode_data = vec![1, 2, 3];
    doc.image_resources = vec![4, 5];
    doc.global_layer_mask = vec![6, 7, 8, 9];
    doc.layer_section_extra = vec![10, 11];
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.color_mode_data, doc.color_mode_data);
    assert_eq!(back.image_resources, doc.image_resources);
    assert_eq!(back.global_layer_mask, doc.global_layer_mask);
    assert_eq!(back.layer_section_extra, doc.layer_section_extra);
    assert_eq!(back, doc);
}

#[test]
fn opaque_layer_blocks_round_trip() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("Opaque", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    layer.blend_key = Some(*b"zzzz");
    layer.blending_ranges = vec![1, 2, 3, 4];
    layer.extra_blocks = vec![LayerBlock {
        key: *b"lfx2",
        data: vec![1, 2, 3, 4],
    }];
    layer.raw_channels = vec![RawChannel {
        id: -3,
        data: vec![0, 0, 9, 9],
    }];
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc);
    assert_eq!(back.layers[0].blend_key, Some(*b"zzzz"));
    assert_eq!(back.layers[0].extra_blocks[0].key, *b"lfx2");
    assert_eq!(back.layers[0].raw_channels[0].data, vec![0, 0, 9, 9]);

    // A changed mode wins over the stale unknown key.
    let mut changed = doc.clone();
    changed.layers[0].blend = BlendMode::Multiply;
    let back = read_psd(&write_psd(&changed).unwrap()).unwrap();
    assert_eq!(back.layers[0].blend_key, None);
}

#[test]
fn recognized_blend_key_is_not_stored() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    let known = pixel("Known", rect(0, 0, 2, 2), 3, BlendMode::Multiply, 255);
    doc.layers = pictura_core::LayerStack::from_layers(vec![known]);
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.layers[0].blend_key, None);
    assert_eq!(back, doc);
}

#[test]
fn mask_extra_round_trips() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("Masked", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    layer.mask = Some(LayerMask {
        rect: rect(0, 0, 2, 2),
        data: Some(vec![9u8; 4].into()),
        extra: vec![7, 8, 9],
        ..Default::default()
    });
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.layers[0].mask.as_ref().unwrap().extra, vec![7, 8, 9]);
    assert_eq!(back, doc);
}

#[test]
fn truncated_preserved_blocks_error() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.global_layer_mask = vec![1, 2, 3, 4];
    let bytes = write_psd(&doc).unwrap();
    // Drop the composite and the last two global-mask bytes.
    let cut = bytes.len() - doc.composite.data.len() - 2 - 2;
    assert!(matches!(read_psd(&bytes[..cut]), Err(PsdError::Truncated)));
}

mod advanced_blending;
mod color_modes;

mod depth;

mod duotone_multichannel;

mod psb;
