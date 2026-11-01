//! PSB write tests, split out of `tests.rs` to stay under the file-size cap.

use super::*;

// -- PSB write ----------------------------------------------------------

fn be_u32(bytes: &[u8], off: usize) -> u32 {
    u32::from_be_bytes(bytes[off..off + 4].try_into().unwrap())
}

fn be_u64(bytes: &[u8], off: usize) -> u64 {
    u64::from_be_bytes(bytes[off..off + 8].try_into().unwrap())
}

/// Offset of the layer-info block just past the header's color-mode data and
/// image-resources sections; returns `(section_len, info_len, info_start)`.
fn layer_lengths(bytes: &[u8]) -> (u64, u64, usize) {
    let mut off = 26;
    off += 4 + be_u32(bytes, off) as usize; // color mode data
    off += 4 + be_u32(bytes, off) as usize; // image resources
    let section_len = be_u64(bytes, off);
    off += 8;
    let info_len = be_u64(bytes, off);
    off += 8;
    (section_len, info_len, off)
}

#[test]
fn write_psb_small_document_widens_lengths() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 3 + 1) as u8;
    }
    doc.layers = pictura_core::LayerStack::from_layers(vec![pixel(
        "One",
        rect(0, 0, 2, 2),
        3,
        BlendMode::Normal,
        255,
    )]);

    let bytes = write_psb(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes([bytes[4], bytes[5]]),
        2,
        "PSB version word"
    );
    let back = read_psd(&bytes).unwrap();
    assert!(back.is_psb, "read back as a PSB");
    doc.is_psb = true;
    assert_eq!(back, doc, "PSB round-trips");

    let (section_len, info_len, info_start) = layer_lengths(&bytes);
    assert!(section_len > 0 && info_len > 0);
    let global_len = be_u32(&bytes, info_start + info_len as usize) as usize;
    // Section covers the 8-byte info prefix, the info block, the 4-byte
    // global-mask prefix, and its bytes (no trailing extra here).
    assert_eq!(section_len, 8 + info_len + 4 + global_len as u64);

    assert_eq!(
        i16::from_be_bytes(bytes[info_start..info_start + 2].try_into().unwrap()),
        1
    );
    let ch_off = info_start + 2 + 16;
    let nch = u16::from_be_bytes(bytes[ch_off..ch_off + 2].try_into().unwrap());
    assert_eq!(nch, 4);
    assert_eq!(
        i16::from_be_bytes(bytes[ch_off + 2..ch_off + 4].try_into().unwrap()),
        0
    );
    assert!(
        be_u64(&bytes, ch_off + 4) >= 2,
        "channel stream has a compression word"
    );
    // Four 10-byte channel entries, then the blend signature: the declared
    // length fields are 8 bytes, not 4.
    let sig = ch_off + 2 + 4 * 10;
    assert_eq!(
        &bytes[sig..sig + 4],
        b"8BIM",
        "declared lengths are 8-byte fields"
    );
}

#[test]
fn write_psd_auto_selects_psb_above_psd_limit() {
    let mut doc = Document::new(30_001, 1, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 5 + 2) as u8;
    }
    let bytes = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes([bytes[4], bytes[5]]),
        2,
        "auto-selected PSB"
    );
    let back = read_psd(&bytes).unwrap();
    assert!(back.is_psb);
    doc.is_psb = true;
    assert_eq!(back, doc, "large document round-trips");
}

#[test]
fn psb_widens_preserved_verbatim_channel_length() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("RawHold", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    let stream = vec![0u8, 3, 1, 2, 3, 0xFF, 0xFE];
    layer.raw_channels = vec![RawChannel {
        id: -3,
        data: stream.clone().into(),
    }];
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let bytes = write_psb(&doc).unwrap();
    let (_, info_len, info_start) = layer_lengths(&bytes);
    let ch_off = info_start + 2 + 16;
    // The preserved channel is the last of five (3 colour + alpha + raw).
    let last = ch_off + 2 + 4 * 10;
    assert_eq!(
        i16::from_be_bytes(bytes[last..last + 2].try_into().unwrap()),
        -3
    );
    let declared = be_u64(&bytes, last + 2);
    assert_eq!(declared as usize, stream.len(), "declared length is u64");
    assert_eq!(&bytes[last + 10..last + 14], b"8BIM");
    assert!(info_len > 0);

    let back = read_psd(&bytes).unwrap();
    assert_eq!(
        back.layers[0].raw_channels[0].data, stream,
        "stream bytes survive"
    );
    doc.is_psb = true;
    assert_eq!(back, doc, "PSB with a preserved channel round-trips");
}

#[test]
fn psb_preserved_big_key_block_uses_u64_length() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("Big", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    layer.extra_blocks = vec![LayerBlock {
        key: *b"Lr16",
        data: vec![1, 2, 3, 4, 5].into(),
    }];
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let bytes = write_psb(&doc).unwrap();
    let sig = bytes
        .windows(8)
        .position(|w| w == b"8BIMLr16")
        .expect("an Lr16 block is present");
    assert_eq!(
        be_u64(&bytes, sig + 8),
        6,
        "a big-key block's length field is 8 bytes and counts the inside pad"
    );
    assert_eq!(&bytes[sig + 16..sig + 22], &[1, 2, 3, 4, 5, 0]);

    let back = read_psd(&bytes).unwrap();
    assert_eq!(back.layers[0].extra_blocks[0].key, *b"Lr16");
    let mut expected = doc.clone();
    expected.is_psb = true;
    expected.layers[0].extra_blocks[0].data = vec![1, 2, 3, 4, 5, 0].into();
    assert_eq!(
        back, expected,
        "preserved big-key block round-trips in a PSB"
    );
}

#[test]
fn psb_reads_tagged_layer_info_signed_8b64() {
    let psd = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rgb16_layered.psd"),
    )
    .unwrap();
    let doc = read_psd(&psd).unwrap();
    let mut bytes = write_psb(&doc).unwrap();
    let sig = bytes
        .windows(8)
        .position(|w| w == b"8BIMLr16")
        .expect("an Lr16 block is present");
    bytes[sig..sig + 4].copy_from_slice(b"8B64");

    let back = read_psd(&bytes).unwrap();
    assert_eq!(back.layers, doc.layers);
    assert!(back.layer_section_extra.is_empty());
}

#[test]
fn psb_document_resaves_as_psb() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 3 + 7) as u8;
    }
    let psb = write_psb(&doc).unwrap();
    let read = read_psd(&psb).unwrap();
    assert!(read.is_psb, "a read PSB is flagged");
    assert!(
        read.width <= 30_000 && read.height <= 30_000,
        "small document"
    );

    let resaved = write_psd(&read).unwrap();
    assert_eq!(
        u16::from_be_bytes([resaved[4], resaved[5]]),
        2,
        "a PSB source re-saves as a PSB even when small"
    );
    assert_eq!(read_psd(&resaved).unwrap(), read, "container is preserved");
}

#[test]
fn write_psb_rejects_dimension_above_psb_limit() {
    let doc = Document::new(300_001, 1, ColorMode::Rgb, BitDepth::Eight);
    assert!(matches!(write_psb(&doc), Err(PsdError::Unsupported(_))));
}

#[test]
fn write_tag_declares_even_length_with_inside_pad() {
    // A 3-byte payload is declared as 4 bytes with the pad inside the length,
    // so psd-tools (padding = 1) does not mistake the pad for a signature.
    let mut out = Vec::new();
    crate::write::write_tag(&mut out, b"lfx2", &[1, 2, 3], false);
    assert_eq!(
        out,
        [b'8', b'B', b'I', b'M', b'l', b'f', b'x', b'2', 0, 0, 0, 4, 1, 2, 3, 0]
    );

    // A PSB big key widens the length to 8 bytes and still pads inside it.
    let mut out = Vec::new();
    crate::write::write_tag(&mut out, b"Lr16", &[9], true);
    assert_eq!(&out[..8], b"8BIMLr16");
    assert_eq!(be_u64(&out, 8), 2);
    assert_eq!(&out[16..], &[9, 0]);
}

#[test]
fn write_tag_document_declares_exact_length_and_pads_to_four() {
    // A global tagged block declares its exact length and is padded externally
    // to a 4-byte boundary, matching psd-tools' padding = 4.
    let mut out = Vec::new();
    crate::write::write_tag_document(&mut out, b"zzzz", &[1, 2, 3, 4, 5], false);
    assert_eq!(&out[..8], b"8BIMzzzz");
    assert_eq!(be_u32(&out, 8), 5, "the declared length is exact");
    assert_eq!(&out[12..], &[1, 2, 3, 4, 5, 0, 0, 0], "external pad to 4");

    // An exact multiple of 4 needs no pad.
    let mut out = Vec::new();
    crate::write::write_tag_document(&mut out, b"zzzz", &[1, 2, 3, 4], false);
    assert_eq!(be_u32(&out, 8), 4);
    assert_eq!(&out[12..], &[1, 2, 3, 4]);

    // A PSB big key widens the length to 8 bytes but still pads externally.
    let mut out = Vec::new();
    crate::write::write_tag_document(&mut out, b"lnk2", &[9, 9, 9, 9, 9, 9], true);
    assert_eq!(&out[..8], b"8BIMlnk2");
    assert_eq!(be_u64(&out, 8), 6);
    assert_eq!(&out[16..], &[9, 9, 9, 9, 9, 9, 0, 0]);
}

#[test]
fn iopa_block_is_four_bytes() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("Fill", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    layer.fill = 128;
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let bytes = write_psd(&doc).unwrap();
    let sig = bytes
        .windows(8)
        .position(|w| w == b"8BIMiOpa")
        .expect("an iOpa block is present");
    assert_eq!(be_u32(&bytes, sig + 8), 4, "iOpa declares 4 bytes");
    assert_eq!(&bytes[sig + 12..sig + 16], &[128, 0, 0, 0]);

    let back = read_psd(&bytes).unwrap();
    assert_eq!(back.layers[0].fill, 128);
}

#[test]
fn psb_composite_uses_u32_rle_counts_and_psd_uses_u16() {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = vec![10, 20, 30].into();
    let psd = write_psd(&doc).unwrap();
    let psb = write_psb(&doc).unwrap();

    // A 1x1 RGB document has three planes, each one pixel -> one packed row of
    // two bytes [control 0, value]. Empty layer/mask section is 4 bytes (PSD)
    // or 8 bytes (PSB), so the image data starts at 38 (PSD) / 42 (PSB).
    assert_eq!(&psd[38..40], &[0, 1], "PSD compression word");
    assert_eq!(&psd[40..46], &[0, 2, 0, 2, 0, 2], "three u16 count entries");
    assert_eq!(&psd[46..], &[0, 10, 0, 20, 0, 30], "PSD packed rows");

    assert_eq!(&psb[42..44], &[0, 1], "PSB compression word");
    assert_eq!(
        &psb[44..56],
        &[0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 2],
        "three u32 count entries"
    );
    assert_eq!(&psb[56..], &[0, 10, 0, 20, 0, 30], "PSB packed rows");
}

#[test]
fn psb_write_is_deterministic() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 11 + 3) as u8;
    }
    doc.layers = pictura_core::LayerStack::from_layers(vec![pixel(
        "One",
        rect(0, 0, 2, 2),
        3,
        BlendMode::Normal,
        255,
    )]);
    assert_eq!(write_psb(&doc).unwrap(), write_psb(&doc).unwrap());
}

#[test]
fn psb_non_big_key_block_keeps_u32_length() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("Small", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    layer.extra_blocks = vec![LayerBlock {
        key: *b"lfx2",
        data: vec![1, 2, 3, 4].into(),
    }];
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let bytes = write_psb(&doc).unwrap();
    let sig = bytes
        .windows(8)
        .position(|w| w == b"8BIMlfx2")
        .expect("an lfx2 block is present");
    assert_eq!(
        be_u32(&bytes, sig + 8),
        4,
        "a non-big-key block stays a 4-byte length in a PSB"
    );
    // If the length were read as u64 it would swallow the payload bytes.
    assert_eq!(&bytes[sig + 12..sig + 16], &[1, 2, 3, 4]);
}

#[test]
fn psb_reframes_preserved_psd_big_key_block() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    // A PSD-framed `lnk2` block: u32 length, even 4-byte payload.
    let mut block = b"8BIMlnk2".to_vec();
    block.extend_from_slice(&4u32.to_be_bytes());
    block.extend_from_slice(&[1, 2, 3, 4]);
    doc.layer_section_extra = block.into();
    doc.is_psb = false;

    let psb = write_psb(&doc).unwrap();
    let sig = psb
        .windows(8)
        .position(|w| w == b"8BIMlnk2")
        .expect("the preserved lnk2 survives");
    assert_eq!(
        be_u64(&psb, sig + 8),
        4,
        "the preserved PSD big-key block is re-framed to a u64 length"
    );
    assert_eq!(&psb[sig + 16..sig + 20], &[1, 2, 3, 4]);

    // A PSD target keeps the original u32 framing byte-for-byte.
    let psd = write_psd(&doc).unwrap();
    assert!(
        psd.windows(doc.layer_section_extra.len())
            .any(|w| w == doc.layer_section_extra.as_slice()),
        "a PSD target copies the preserved block unchanged"
    );
}
