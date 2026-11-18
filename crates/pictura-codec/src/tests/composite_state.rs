use super::*;

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
    doc.composite.set_merged_output(false);

    let bytes = write_psd(&doc).unwrap();
    // No image-data section: the file ends at the layer/mask section.
    let section_len = u32::from_be_bytes(bytes[34..38].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), 38 + section_len, "no image-data section");

    let back = read_psd(&bytes).unwrap();
    assert!(!back.composite.merged_output());
    assert_eq!(back, doc);
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
    assert!(!back.composite.merged_output());

    // A bare header with no layer section still errors as truncated.
    assert!(read_psd(&header(1, 3, 1, 1, 3)).is_err());
}

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
    assert!(doc.composite.merged_output());
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
