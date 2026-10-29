use super::*;

#[test]
fn an_empty_layer_or_an_empty_selected_area_has_nothing_to_filter() {
    use crate::cxxqt_object::tests::pixel_layer;
    use pictura_select::Selection;

    let mut layer = pixel_layer("layer", 4, 4, (10, 20, 30));
    assert!(!selected_area_empty(&layer, None));
    let mut no_alpha = layer.clone();
    no_alpha.channels.retain(|c| c.id != -1);
    assert!(!selected_area_empty(&no_alpha, None), "an opaque layer");

    // One opaque pixel at (3, 1); the rest transparent.
    let mut alpha = vec![0u8; 16];
    alpha[4 + 3] = 255;
    layer.channels.last_mut().unwrap().data = alpha.into();
    assert!(!selected_area_empty(&layer, None));
    let mut selection = Selection {
        width: 4,
        height: 4,
        data: vec![0; 16].into(),
    };
    selection.data[0] = 255;
    assert!(
        selected_area_empty(&layer, Some(&selection)),
        "selection misses it"
    );
    selection.data[4 + 3] = 1;
    assert!(!selected_area_empty(&layer, Some(&selection)));

    layer.channels.last_mut().unwrap().data = vec![0u8; 16].into();
    assert!(selected_area_empty(&layer, None), "fully transparent");
    let mut sized_out = pixel_layer("new", 0, 0, (0, 0, 0));
    assert!(
        selected_area_empty(&sized_out, None),
        "a zero-size new layer"
    );
    sized_out.channels.clear();
    assert!(selected_area_empty(&sized_out, None));
}

#[test]
fn a_stack_takes_each_kinds_slots_in_turn() {
    let mut kinds = QStringList::default();
    kinds.append(QString::from("cutout"));
    kinds.append(QString::from("palette-knife"));
    let mut params = QList::<f64>::default();
    for v in [4.0, 4.0, 2.0, 25.0, 3.0, 0.0] {
        params.append(v);
    }
    let resolve = |params: &QList<f64>| {
        let params: Vec<f64> = params.into_iter().copied().collect();
        resolve_stack(&kinds, &params)
    };
    let stack = resolve(&params).expect("stack resolves");
    assert_eq!(stack.len(), 2);
    assert_eq!(
        stack[1],
        pictura_filters::Filter::PaletteKnife {
            stroke_size: 25,
            stroke_detail: 3,
            softness: 0,
        }
    );
    // One slot short, or one over, is refused rather than misread.
    params.remove(5);
    assert!(resolve(&params).is_none());
    params.append(0.0);
    params.append(0.0);
    assert!(resolve(&params).is_none());
}

#[test]
fn rgba_round_trips_through_a_planar_buffer() {
    let rgba = [1u8, 2, 3, 4, 5, 6, 7, 8];
    let buffer = rgba_to_buffer(&rgba, 2, 1).expect("buffer");
    assert_eq!(buffer.data[0], 1);
    assert_eq!(buffer.data[1], 5);
    assert_eq!(buffer.data[2 * 2 + 1], 7);
    assert_eq!(buffer.data[3 * 2], 4);
    assert!(rgba_to_buffer(&rgba, 3, 1).is_none());
}

#[test]
fn the_proxy_base_hides_the_active_layer_and_restores_it() {
    use crate::cxxqt_object::tests::pixel_layer;
    use pictura_core::{BitDepth, ColorMode};

    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("bottom", 4, 4, (255, 0, 0)),
        pixel_layer("top", 4, 4, (0, 0, 255)),
    ]);
    doc.composite = pictura_render::composite_rgba(&doc);

    let base = hidden_layer_buffer(&mut doc, "1", false).expect("base");
    assert!(doc.layers[1].visible, "hiding must be undone");
    let plane = 16;
    assert_eq!(base.data[0], 255, "the red bottom layer");
    assert_eq!(base.data[2 * plane], 0, "the blue top layer is still in");
    assert_eq!(base.data[3 * plane], 255);
}

#[test]
fn the_proxy_layer_filters_and_positions_only_the_active_layer() {
    use crate::cxxqt_object::tests::pixel_layer;
    use pictura_core::{BitDepth, ColorMode, PsdRect};

    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    let mut top = pixel_layer("top", 2, 2, (0, 0, 255));
    top.rect = PsdRect {
        top: 2,
        left: 2,
        bottom: 4,
        right: 4,
    };
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("bottom", 8, 8, (255, 0, 0)), top]);

    let frame =
        proxy_layer_frame_buffer(&doc.layers[1], 8, 8, 8, "gaussian-blur", &[1.0]).expect("frame");
    let plane = 64;
    let at = |x: usize, y: usize| y * 8 + x;
    // Inside the layer's rect the uniform blue survives its blur...
    assert_eq!(frame.data[at(2, 2)], 0);
    assert_eq!(frame.data[plane + at(2, 2)], 0);
    assert_eq!(frame.data[2 * plane + at(2, 2)], 255);
    assert_eq!(frame.data[3 * plane + at(2, 2)], 255);
    // ...and everywhere else the frame is transparent: the bottom layer
    // and the unfiltered position are not drawn.
    assert_eq!(frame.data[3 * plane + at(0, 0)], 0);
    assert_eq!(frame.data[3 * plane + at(5, 5)], 0);
}

#[test]
fn the_proxy_layer_bakes_opacity_and_mask_into_the_alpha() {
    use crate::cxxqt_object::tests::pixel_layer;

    let mut layer = pixel_layer("top", 2, 2, (0, 0, 255));
    layer.opacity = 128;
    let buffer = proxy_layer_buffer(&layer, 1.0).expect("layer buffer");
    assert_eq!(buffer.data[3 * 4], 128, "opacity baked into the alpha");
}

#[test]
fn a_nested_active_layer_resolves_for_the_proxy() {
    use crate::cxxqt_object::tests::pixel_layer;
    use pictura_core::{BitDepth, ColorMode};

    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut group = pixel_layer("group", 4, 4, (0, 0, 0));
    group.is_group = true;
    group.channels.clear();
    group.children = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("nested-red", 4, 4, (255, 0, 0)),
        pixel_layer("nested-blue", 4, 4, (0, 0, 255)),
    ]);
    doc.layers = pictura_core::LayerStack::from_layers(vec![group]);
    doc.composite = pictura_render::composite_rgba(&doc);

    // The group itself is not a proxy target; the nested leaf is.
    assert!(active_proxy_layer(&doc, Some("0")).is_none());
    assert!(active_proxy_layer(&doc, Some("0/1")).is_some());

    let base = hidden_layer_buffer(&mut doc, "0/1", false).expect("nested base");
    let plane = 16;
    assert_eq!(base.data[0], 255, "the red nested layer survives");
    assert_eq!(base.data[2 * plane], 0, "the blue nested layer is hidden");
    assert!(doc.layers[0].children[1].visible, "visibility restored");
}

#[test]
fn filter_descriptors_mirror_the_catalogue() {
    let rows = filter_descriptors();
    let specs: Vec<_> = pictura_filters::catalogue::all().collect();
    assert_eq!(rows.len(), specs.len());
    for (row, spec) in rows.iter().zip(specs) {
        assert_eq!(row.kind.to_string(), spec.id);
        assert_eq!(row.menu, spec.menu);
        assert_eq!(row.label.to_string(), spec.label);
        assert_eq!(row.gallery_rank, spec.gallery.map_or(-1, i32::from));
        let path: Vec<String> = row.path.iter().map(ToString::to_string).collect();
        let expected: Vec<String> = spec.path.iter().map(|part| (*part).to_string()).collect();
        assert_eq!(path, expected);
        assert_eq!(row.params.len(), spec.params.len());
        for (param, source) in row.params.iter().zip(spec.params) {
            assert_eq!(param.control, source.control as i32);
            assert_eq!(param.minimum, source.minimum);
            assert_eq!(param.maximum, source.maximum);
            assert_eq!(param.value, source.value);
            assert_eq!(param.slot_count, source.slots() as i32);
            assert_eq!(param.measures_pixels, source.pixel_distance);
        }
    }
}

#[test]
fn hdr_toning_is_not_a_filter_command() {
    assert!(!filter_kind_supported(&cxx_qt_lib::QString::from(
        "hdr-toning"
    )));
    assert_eq!(
        filter_param_arity(&cxx_qt_lib::QString::from("hdr-toning")),
        0
    );
    assert!(pictura_filters::catalogue::descriptor("hdr-toning").is_some());
}
