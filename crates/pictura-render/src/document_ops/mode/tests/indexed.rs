use super::*;

fn indexed_transparent_source() -> Document {
    Document::from_rgba("Source", 2, 1, &[49, 23, 17, 255, 46, 23, 17, 0])
}

#[test]
fn forced_indexed_transparency_round_trips() {
    let mut doc = indexed_transparent_source();
    let mut options = indexed_options(ColorReduction::Adaptive, Dither::None);
    options.transparency = true;
    convert_to_indexed_with_forced(&mut doc, options, &[[0; 3], [255; 3]], false).unwrap();
    let reopened = roundtrip(&doc);
    let table = indexed_color_table(&reopened).unwrap();
    assert!(table.colors.contains(&[0; 3]));
    assert!(table.colors.contains(&[255; 3]));
    assert_ne!(table.transparent, None);
    assert_eq!(layer_plane(&reopened, -1), vec![255, 0]);
}

#[test]
fn indexed_conversion_without_transparency_uses_white() {
    let mut doc = indexed_transparent_source();
    let options = indexed_options(ColorReduction::Adaptive, Dither::None);
    convert_to_indexed(&mut doc, options).unwrap();
    assert_eq!(layer_plane(&doc, 0), vec![49, 255]);
    assert_eq!(layer_plane(&doc, 1), vec![23, 255]);
    assert_eq!(layer_plane(&doc, -1), vec![255, 255]);
    assert_eq!(indexed_color_table(&doc).unwrap().transparent, None);
}

#[test]
fn forced_indexed_transparency_capacity_boundary() {
    let mut doc = indexed_transparent_source();
    let mut options = indexed_options(ColorReduction::Adaptive, Dither::None);
    options.transparency = true;
    let forced: Vec<_> = (0..=u8::MAX).map(|v| [v, 1, 2]).collect();
    let original = doc.clone();
    assert!(convert_to_indexed_with_forced(&mut doc, options, &forced, false).is_err());
    assert_eq!(doc, original);
    convert_to_indexed_with_forced(&mut doc, options, &forced[..255], false).unwrap();
    let table = indexed_color_table(&doc).unwrap();
    assert_eq!(table.transparent, Some(255));
    assert_eq!(&table.colors[..255], &forced[..255]);
}

#[test]
fn invalid_exact_request_is_atomic() {
    let mut doc = indexed_transparent_source();
    let before = doc.clone();
    let mut options = indexed_options(ColorReduction::Adaptive, Dither::None);
    options.transparency = true;
    let forced: Vec<_> = (0..=254).map(|v| [v, 1, 2]).collect();
    assert!(!indexed_exact_fits(&doc, options, &forced));
    assert!(convert_to_indexed_with_forced(&mut doc, options, &forced, true).is_err());
    assert_eq!(doc, before);
}

#[test]
fn exact_indexed_conversion_fits_capacity() {
    let mut doc = indexed_transparent_source();
    let mut options = indexed_options(ColorReduction::Adaptive, Dither::None);
    options.transparency = true;
    let forced: Vec<_> = (0..=253).map(|v| [v, 1, 2]).collect();
    assert!(indexed_exact_fits(&doc, options, &forced));
    convert_to_indexed_with_forced(&mut doc, options, &forced, true).unwrap();
    let table = indexed_color_table(&doc).unwrap();
    assert_eq!(table.colors.len(), 256);
    assert_eq!(table.transparent, Some(255));
    assert!(table.colors.contains(&[49, 23, 17]));
    assert_eq!(layer_plane(&doc, 0)[0], 49);
}

#[test]
fn color_table_replacement_preserves_indices() {
    let mut doc = indexed_transparent_source();
    let options = indexed_options(ColorReduction::Adaptive, Dither::None);
    convert_to_indexed(&mut doc, options).unwrap();
    let original = doc.source.planes().clone();
    let mut table = indexed_color_table(&doc).unwrap();
    table.colors[0] = [10, 20, 30];
    table.transparent = Some(1);
    assert!(replace_indexed_color_table(&mut doc, table.clone()).unwrap());
    assert_eq!(doc.source.planes(), &original);
    assert_eq!(layer_plane(&doc, 0)[0], 10);
    assert_eq!(layer_plane(&doc, -1), vec![255, 0]);
    assert_eq!(indexed_color_table(&doc).unwrap(), table);
}

#[test]
fn flat_grayscale_alpha_converts_to_indexed_with_and_without_transparency() {
    let mut source = Document::new(2, 1, ColorMode::Grayscale, BitDepth::Eight);
    source
        .composite
        .replace(PixelBuffer {
            width: 2,
            height: 1,
            channels: 2,
            data: vec![40, 80, 255, 0].into(),
        })
        .unwrap();
    for transparency in [false, true] {
        let mut doc = source.clone();
        let mut options = indexed_options(ColorReduction::Adaptive, Dither::None);
        options.transparency = transparency;
        assert!(indexed_exact_fits(&doc, options, &[]));
        convert_to_indexed_with_forced(&mut doc, options, &[], true).unwrap();
        assert_eq!(layer_plane(&doc, 0)[0], 40);
        assert_eq!(
            layer_plane(&doc, -1),
            if transparency {
                vec![255, 0]
            } else {
                vec![255, 255]
            }
        );
        if !transparency {
            assert_eq!(layer_plane(&doc, 0)[1], 255);
        }
    }
}
