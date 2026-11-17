use super::*;
use pictura_core::{FillContent, LayerBlock, LayerKind, LayerParts, TypeSpec};

fn rectangle() -> pictura_core::path::Subpath {
    pictura_core::path::Subpath {
        closed: true,
        points: [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)].into_iter().map(|anchor| pictura_core::path::PathPoint { anchor, in_handle: None, out_handle: None, smooth: false }).collect(),
    }
}

fn reopen(doc: &Document) -> Document {
    pictura_codec::read_psd(&pictura_codec::write_psd(doc).unwrap()).unwrap()
}

#[test]
fn edited_fill_reopens_with_new_parameters() {
    let unrelated = LayerBlock { key: *b"zzzz", data: vec![19, 23, 29].into() };
    let source = pictura_codec::layer_from_parts(LayerParts {
        rect: full(4, 4), adjustment: Some(encode_solid_color_fill([10, 20, 30])),
        extra_blocks: vec![unrelated.clone()], ..Default::default()
    });
    let mut d = reopen(&doc(4, 4, vec![source]));
    let previous = composite_rgba(&d);
    let clone = d.clone();
    assert!(d.layers[0].set_fill_content(FillContent::Solid([40, 50, 60, 255])));
    let edited = composite_rgba(&d);
    assert_ne!(edited, previous);
    assert_eq!(composite_rgba(&clone), previous);
    for y in 0..4 { for x in 0..4 { assert_eq!(px(&edited, x, y), [40, 50, 60, 255]); } }
    let back = reopen(&d);
    assert_eq!(back.layers[0].kind(), LayerKind::Fill);
    assert_eq!(back.layers[0].fill_content(), Some(&FillContent::Solid([40, 50, 60, 255])));
    assert_eq!(back.layers[0].extra_block(b"zzzz"), Some(&unrelated));
    let picture = composite_rgba(&back);
    for y in 0..4 { for x in 0..4 { assert_eq!(px(&picture, x, y), [40, 50, 60, 255]); } }
}

#[test]
fn rasterized_content_remains_pixel_after_reopen() {
    let mut fill = doc(16, 16, vec![]);
    let fill_path = add_solid_fill(&mut fill, "", [10, 20, 30, 255]);
    let mut shape = doc(16, 16, vec![]);
    let shape_path = add_shape_layer(&mut shape, "", [10, 20, 30, 255], "shape", &rectangle(), None);
    let mut smart = doc(16, 16, vec![solid("smart", full(16, 16), (10, 20, 30), 255, BlendMode::Normal, 255)]);
    assert!(convert_to_smart_object(&mut smart, "0"));
    let mut text = doc(16, 16, vec![]);
    let mut spec = TypeSpec::new("A", "DejaVu Sans", 8.0); spec.origin = (2.0, 10.0);
    let text_path = add_type_layer(&mut text, "", &spec);
    let mut source_free = Layer::new(pictura_core::LayerContent::Fill(std::sync::Arc::new(FillContent::Solid([10, 20, 30, 123]))));
    source_free.rect = full(16, 16); source_free.name = "typed fill".into();
    assert!(source_free.adjustment().is_none() && source_free.extra_blocks().is_empty());
    let source_free = doc(16, 16, vec![source_free]);
    let source_free_picture = composite_rgba(&source_free);
    for y in 0..16 { for x in 0..16 { assert_eq!(px(&source_free_picture, x, y), [10, 20, 30, 123]); } }
    for (mut d, path, kind) in [(source_free, "0".to_owned(), LayerKind::Fill), (fill, fill_path, LayerKind::Fill), (shape, shape_path, LayerKind::Shape), (text, text_path, LayerKind::Text), (smart, "0".to_owned(), LayerKind::SmartObject)] {
        assert!(!path.is_empty());
        let before = composite_rgba(&d);
        let saved_name = resolve_path(&d, &path).unwrap().name.clone();
        let applied = match kind {
            LayerKind::Fill => rasterize_fill_content(&mut d, &path),
            LayerKind::Shape => rasterize_shape(&mut d, &path),
            LayerKind::Text => render_text_layer(&mut d, &path),
            LayerKind::SmartObject => rasterize_smart_object(&mut d, &path),
            _ => unreachable!(),
        };
        assert!(applied, "{kind:?}");
        let layer = resolve_path(&d, &path).unwrap();
        assert_eq!(layer.kind(), LayerKind::Pixel);
        assert_eq!(layer.name, saved_name);
        for key in [b"TySh", b"SoLd", b"SoLE", b"plLd", b"PlLd", b"vscg"] { assert!(layer.extra_block(key).is_none(), "{kind:?} {key:?}"); }
        let back = reopen(&d);
        assert_eq!(resolve_path(&back, &path).unwrap().kind(), LayerKind::Pixel);
        assert_eq!(composite_rgba(&d), before, "{kind:?}");
        assert_eq!(composite_rgba(&back), before, "{kind:?} reopen");
    }
}

#[test]
fn vector_masked_pixels_do_not_become_shapes() {
    let mut d = doc(4, 4, vec![solid("pixel", full(4, 4), (10, 20, 30), 255, BlendMode::Normal, 255)]);
    assert!(add_vector_mask(&mut d, "0", VectorMaskKind::RevealAll));
    assert_eq!(d.layers[0].kind(), LayerKind::Pixel);
    assert!(!is_shape_layer(&d.layers[0]));
    let before = d.clone();
    assert!(!rasterize_shape(&mut d, "0"));
    assert_eq!(d, before);
    assert_eq!(reopen(&d).layers[0].kind(), LayerKind::Pixel);
    assert!(delete_vector_mask(&mut d, "0"));
    assert_eq!(d.layers[0].kind(), LayerKind::Pixel);
}

#[test]
fn vector_mask_changes_transition_between_fill_and_shape() {
    for rasterize in [false, true] {
        let mut d = doc(4, 4, vec![]);
        let path = add_solid_fill(&mut d, "", [10, 20, 30, 255]);
        assert_eq!(d.layers[0].kind(), LayerKind::Fill);
        let flag = d.layers[0].pixel_data_irrelevant;
        assert!(add_vector_mask(&mut d, &path, VectorMaskKind::RevealAll));
        assert_eq!(d.layers[0].kind(), LayerKind::Shape);
        assert!(is_shape_layer(&d.layers[0]));
        assert!(set_vector_mask_enabled(&mut d, &path, false));
        assert_eq!(d.layers[0].kind(), LayerKind::Shape);
        assert!(set_vector_mask_enabled(&mut d, &path, true));
        d.layers[0].mask = Some(LayerMask { rect: full(4, 4), data: Some(vec![128; 16].into()), ..Default::default() });
        let before = composite_rgba(&d);
        assert_eq!(reopen(&d).layers[0].kind(), LayerKind::Shape);
        assert!(if rasterize { rasterize_vector_mask(&mut d, &path) } else { delete_vector_mask(&mut d, &path) });
        assert_eq!(d.layers[0].kind(), LayerKind::Fill);
        assert!(!is_shape_layer(&d.layers[0]));
        assert!(d.layers[0].extra_block(b"vmsk").is_none());
        assert!(d.layers[0].extra_block(b"vsms").is_none());
        assert_eq!(d.layers[0].pixel_data_irrelevant, flag);
        assert_eq!(composite_rgba(&d), before);
        let back = reopen(&d);
        assert_eq!(back.layers[0].kind(), LayerKind::Fill);
        assert_eq!(back.layers[0].pixel_data_irrelevant, flag);
        assert_eq!(composite_rgba(&back), before);
    }
    let mask = pictura_codec::encode_vector_mask(&[rectangle()], 4, 4);
    for (keys, parsed) in [(vec![(*b"vsms", mask.clone())], true), (vec![(*b"vmsk", vec![255; 3]), (*b"vsms", mask.clone())], false)] {
        let layer = pictura_codec::layer_from_parts(LayerParts { rect: full(4, 4), adjustment: Some(encode_solid_color_fill([10, 20, 30])), extra_blocks: keys.into_iter().map(|(key, data)| LayerBlock { key, data: data.into() }).collect(), ..Default::default() });
        let mut d = doc(4, 4, vec![layer]);
        assert_eq!(d.layers[0].kind(), LayerKind::Shape);
        assert_eq!(d.layers[0].vector_mask.is_some(), parsed);
        if !parsed {
            let before = d.clone();
            assert!(!delete_vector_mask(&mut d, "0"));
            assert!(!rasterize_vector_mask(&mut d, "0"));
            assert_eq!(d, before);
        }
    }
}

#[test]
fn fill_encoders_preserve_descriptor_contract() {
    use pictura_core::{GradientFillParams, GradientKind, GradientStop, PatternFillParams};
    let solid = encode_solid_color_fill([10, 20, 30]);
    assert_eq!(&solid.data[..4], &16u32.to_be_bytes());
    assert_eq!(pictura_codec::decode_fill(&solid), Some(FillContent::Solid([10, 20, 30, 255])));
    assert_eq!(pictura_codec::decode_fill(&adjdata(*b"SoCo", vec![10, 20, 30, 40])), Some(FillContent::Solid([10, 20, 30, 40])));
    let mut gradient = GradientFillParams { stops: vec![GradientStop { location: 0, color: [0; 3] }, GradientStop { location: 4096, color: [255; 3] }], reverse: true, kind: GradientKind::Radial, angle_deg: 45.0, scale: 60.0 };
    assert_eq!(pictura_codec::decode_fill(&encode_gradient_fill_params(&gradient)), Some(FillContent::Gradient(gradient.clone())));
    gradient.reverse = false; gradient.scale = 100.0;
    let encoded = encode_gradient_fill_params(&gradient);
    let DescValue::Object { items, .. } = pictura_codec::read_descriptor(&encoded.data).unwrap() else { panic!("descriptor"); };
    assert!(!items.iter().any(|(key, _)| key == b"Rvrs" || key == b"Scl "));
    let mut pattern = PatternFillParams { pattern_id: "p1".into(), scale: 50.0, link_with_layer: false, origin: (3, 4) };
    assert_eq!(pictura_codec::decode_fill(&encode_pattern_fill(&pattern)), Some(FillContent::Pattern(pattern.clone())));
    pattern.origin = (0, 0);
    let encoded = encode_pattern_fill(&pattern);
    let DescValue::Object { items, .. } = pictura_codec::read_descriptor(&encoded.data).unwrap() else { panic!("descriptor"); };
    assert!(!items.iter().any(|(key, _)| key == b"phase"));
}
