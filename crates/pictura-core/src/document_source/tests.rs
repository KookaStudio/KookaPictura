use crate::*;

fn store(depth: BitDepth) -> SourcePlanes {
    let samples = if depth == BitDepth::One {
        Samples::U8(vec![0x80].into())
    } else {
        Samples::widen_from_u8(&[17, 31, 90], depth)
    };
    SourcePlanes {
        depth,
        width: 1,
        height: 1,
        samples,
    }
}

fn native_layer(depth: BitDepth) -> Layer {
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 1,
        right: 1,
    };
    Layer::from_parts(
        LayerParts {
            rect,
            source_channels: Some(SourceChannels::new(
                depth,
                rect,
                vec![(0, Samples::widen_from_u8(&[17], depth))],
            )),
            ..Default::default()
        },
        LayerContent::Pixel,
    )
}

#[test]
fn source_state_legal_cases() {
    for working in [ColorMode::Rgb, ColorMode::Grayscale] {
        for depth in [None, Some(BitDepth::Sixteen), Some(BitDepth::ThirtyTwo)] {
            for retained in [false, true] {
                let planes = depth.filter(|_| retained).map(store);
                let mut doc = Document::new(1, 1, working, BitDepth::Eight);
                doc.source = SourceState::new(None, depth, planes, None).unwrap();
                assert_eq!(doc.document_color_mode(), working);
                assert_eq!(doc.retains_source_depth(), retained && depth.is_some());
                assert_eq!(
                    doc.document_bit_depth(),
                    depth.filter(|_| retained).unwrap_or(BitDepth::Eight)
                );
                for output in [false, true] {
                    doc.composite.set_merged_output(output);
                    assert!(doc.has_stored_composite());
                    let data = doc.composite.detach_plane();
                    assert!(!doc.has_stored_composite());
                    assert_eq!(doc.composite.merged_output(), output);
                    doc.composite.attach_plane(data).unwrap();
                    assert!(doc.has_stored_composite());
                }
            }
        }
    }
    for mode in [
        ColorMode::Bitmap,
        ColorMode::Indexed,
        ColorMode::Cmyk,
        ColorMode::Lab,
        ColorMode::Duotone,
        ColorMode::Multichannel,
    ] {
        let palette = (mode == ColorMode::Indexed).then_some([0; 768]);
        let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
        doc.source = SourceState::new(Some(mode), None, None, palette).unwrap();
        assert_eq!(doc.document_color_mode(), mode);
        assert_eq!(
            doc.document_bit_depth(),
            if mode == ColorMode::Bitmap {
                BitDepth::One
            } else {
                BitDepth::Eight
            }
        );
        if matches!(
            mode,
            ColorMode::Bitmap | ColorMode::Indexed | ColorMode::Lab | ColorMode::Cmyk
        ) {
            let depth = if mode == ColorMode::Bitmap {
                BitDepth::One
            } else {
                BitDepth::Eight
            };
            doc.source.set_native(None, Some(store(depth))).unwrap();
            assert!(doc.source.planes().is_some());
        }
    }
    for mode in [ColorMode::Cmyk, ColorMode::Lab] {
        for depth in [BitDepth::Sixteen, BitDepth::ThirtyTwo] {
            let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
            doc.source =
                SourceState::new(Some(mode), Some(depth), Some(store(depth)), None).unwrap();
            assert_eq!(doc.document_color_mode(), mode);
            assert_eq!(doc.document_bit_depth(), depth);
            assert!(doc.retains_source_depth());
            assert!(doc.authoritative_plane(&native_layer(depth), 0).is_none());
        }
    }
}

#[test]
fn source_state_rejects_invalid_combinations() {
    assert!(SourceState::new(Some(ColorMode::Indexed), None, None, None).is_err());
    assert!(SourceState::new(None, None, None, Some([0; 768])).is_err());
    let mut bad = store(BitDepth::Sixteen);
    bad.samples = Samples::F32(vec![0.5; 3].into());
    assert!(SourceState::new(None, Some(BitDepth::Sixteen), Some(bad), None).is_err());
}

#[test]
fn source_queries_include_descendant_stores() {
    for depth in [BitDepth::Sixteen, BitDepth::ThirtyTwo] {
        let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(vec![Layer::new(LayerContent::Group(
            LayerStack::from_layers(vec![native_layer(depth)]),
        ))]);
        assert!(doc.retains_source_depth());
        assert!(doc
            .authoritative_plane(&doc.layers[0].children()[0], 0)
            .is_none());
        doc.source.set_native(Some(depth), None).unwrap();
        assert!(doc
            .authoritative_plane(&doc.layers[0].children()[0], 0)
            .is_some());
        doc.source
            .set_encoding(Some(ColorMode::Cmyk), None)
            .unwrap();
        assert!(doc.retains_source_depth());
        assert!(doc
            .authoritative_plane(&doc.layers[0].children()[0], 0)
            .is_none());
        doc.layers[0].children_mut().expect("group")[0].source_channels = None;
        assert!(!doc.retains_source_depth());
    }
}

#[test]
fn native_plane_authority_preserves_fallbacks() {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = native_layer(BitDepth::Sixteen);
    assert!(doc.authoritative_plane(&layer, 0).is_none());
    doc.source
        .set_native(Some(BitDepth::ThirtyTwo), None)
        .unwrap();
    assert!(doc.authoritative_plane(&layer, 0).is_none());
    doc.source
        .set_native(Some(BitDepth::Sixteen), None)
        .unwrap();
    assert!(doc.authoritative_plane(&layer, 0).is_some());
    assert!(doc.authoritative_plane(&layer, -1).is_none());
    layer.rect.left = -1;
    assert!(doc.authoritative_plane(&layer, 0).is_none());
    layer.rect.left = 0;
    doc.source.set_encoding(Some(ColorMode::Lab), None).unwrap();
    assert!(doc.authoritative_plane(&layer, 0).is_none());
}

#[test]
fn composite_roles_and_output_policy_are_independent() {
    let mut doc = Document::new(1, 1, ColorMode::Grayscale, BitDepth::Eight);
    assert!(doc.has_stored_composite());
    assert!(!doc.composite_shape_is_valid());
    assert!(doc.composite_is_authoritative());
    doc.composite.set_merged_output(false);
    assert!(doc.has_stored_composite());
    let data = doc.composite.detach_plane();
    assert!(!doc.has_stored_composite());
    assert!(!doc.composite.merged_output());
    doc.composite.attach_plane(data).unwrap();
    doc.mode = ColorMode::Rgb;
    doc.composite.replace(PixelBuffer::new(1, 1, 4)).unwrap();
    assert!(doc.composite_shape_is_valid());
    doc.layers = LayerStack::from_layers(vec![Layer::default()]);
    assert!(!doc.composite_is_authoritative());
    doc.width = 2;
    assert!(!doc.has_stored_composite());
    assert!(!doc.composite_shape_is_valid());
}

#[test]
fn state_replacement_refusal_is_atomic() {
    let mut doc = Document::from_rgba("pixel", 1, 1, &[10, 20, 30, 255]);
    let before = doc.clone();
    assert!(doc
        .source
        .set_native(Some(BitDepth::Sixteen), Some(store(BitDepth::ThirtyTwo)))
        .is_err());
    assert_eq!(doc, before);
    assert!(doc
        .composite
        .replace(PixelBuffer {
            width: 1,
            height: 1,
            channels: 4,
            data: vec![7; 3].into()
        })
        .is_err());
    assert_eq!(doc, before);
    assert!(doc.composite.data.shares(&before.composite.data));
    assert!(doc.composite.edit(|buffer| buffer.channels = 1).is_err());
    assert_eq!(doc, before);
}

#[test]
fn sample_replacement_preserves_metadata_and_rejects_shape_or_type_changes() {
    let mut source = SourceState::new(
        None,
        Some(BitDepth::Sixteen),
        Some(store(BitDepth::Sixteen)),
        None,
    )
    .unwrap();
    let before = source.clone();
    assert!(source
        .replace_samples(Samples::F32(vec![0.5; 3].into()))
        .is_err());
    assert!(source
        .replace_samples(Samples::U16(vec![1; 2].into()))
        .is_err());
    assert_eq!(source, before);
    source
        .replace_samples(Samples::U16(vec![1, 2, 3].into()))
        .unwrap();
    let result = source.planes().as_ref().unwrap();
    assert_eq!(
        (result.width, result.height, result.depth),
        (1, 1, BitDepth::Sixteen)
    );
    assert_eq!(result.samples, Samples::U16(vec![1, 2, 3].into()));
}

#[test]
fn composite_fixed_slice_edits_keep_unique_allocation_and_fork_shared_data() {
    let mut state = CompositeState::new(PixelBuffer {
        width: 1,
        height: 1,
        channels: 3,
        data: vec![1, 2, 3].into(),
    })
    .unwrap();
    let address = state.data.as_ptr();
    state.edit_data_slice(|data| data[0] = 7);
    assert_eq!(state.data.as_ptr(), address);
    let snapshot = state.clone();
    state.edit_data_slice(|data| data[1] = 8);
    assert_eq!(snapshot.data.as_slice(), &[7, 2, 3]);
    assert_eq!(state.data.as_slice(), &[7, 8, 3]);
    assert!(!state.data.shares(&snapshot.data));
}
