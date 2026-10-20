use super::*;

#[test]
fn appends_topmost_native_size_layer() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    {
        let _ = doc.layers.push(solid(
            "base",
            full(4, 4),
            (0, 0, 0),
            255,
            BlendMode::Normal,
            255,
        ));
    };
    let rgba: Vec<u8> = vec![
        10, 20, 30, 255, 40, 50, 60, 128, 70, 80, 90, 255, 100, 110, 120, 255,
    ];
    let path = add_raster_layer_from_rgba(&mut doc, "img", 2, 2, &rgba);
    assert_eq!(path, "1");
    assert_eq!(doc.layers.len(), 2);
    let layer = resolve_path(&doc, &path).expect("the returned path resolves");
    assert_eq!(layer.name, "img");
    assert_eq!((layer.rect.width(), layer.rect.height()), (2, 2));
    assert!(layer.visible);
    assert_eq!(layer.blend, BlendMode::Normal);
    assert_eq!(layer.opacity, 255);
    assert_eq!(layer.fill, 255);
    assert!(layer.smart_object.is_none());
}

#[test]
fn pixels_are_planar_and_faithful() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    let rgba = [255u8, 0, 0, 255, 0, 255, 0, 128];
    let path = add_raster_layer_from_rgba(&mut doc, "px", 2, 1, &rgba);
    let layer = resolve_path(&doc, &path).expect("path resolves");
    let channel = |id: i16| {
        &layer
            .channels
            .iter()
            .find(|c| c.id == id)
            .expect("channel present")
            .data[..]
    };
    assert_eq!(channel(0), &[255, 0]);
    assert_eq!(channel(1), &[0, 255]);
    assert_eq!(channel(2), &[0, 0]);
    assert_eq!(channel(-1), &[255, 128]);
}

#[test]
fn zero_dimension_is_refused_without_mutation() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let before = doc.clone();
    assert!(add_raster_layer_from_rgba(&mut doc, "z", 0, 4, &[0; 16]).is_empty());
    assert!(add_raster_layer_from_rgba(&mut doc, "z", 4, 0, &[0; 16]).is_empty());
    assert_eq!(doc, before);
}

#[test]
fn appended_layer_converts_to_smart_object() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    let path = add_raster_layer_from_rgba(&mut doc, "img", 2, 2, &[200; 16]);
    assert!(convert_to_smart_object(&mut doc, &path));
    let layer = resolve_path(&doc, &path).expect("path resolves");
    assert!(layer.smart_object.is_some());
}
