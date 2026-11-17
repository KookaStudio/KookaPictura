use super::*;

#[test]
fn selection_becomes_full_frame_mask_that_confines_adjustment() {
    let mut doc = Document::new(4, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("base", 4, 1, (255, 0, 0))]);

    let selection = Selection {
        width: 4,
        height: 1,
        data: vec![255, 255, 0, 0].into(),
    };
    let mask = default_layer_mask(&doc, Some(selection.coverage()));
    assert_eq!(mask.rect.right, 4);
    assert_eq!(mask.rect.bottom, 1);
    assert_eq!(mask.data.as_deref(), Some(&[255u8, 255, 0, 0][..]));

    let _ = doc
        .layers
        .push(adjustment_layer("invert", Some(mask)).expect("known kind"));
    let image = document_to_image(&doc, false);
    // Selected half inverts red -> cyan; unselected half is untouched.
    assert_eq!(image.pixel_color(0, 0).red(), 0);
    assert_eq!(image.pixel_color(0, 0).green(), 255);
    assert_eq!(image.pixel_color(0, 0).blue(), 255);
    assert_eq!(image.pixel_color(3, 0).red(), 255);
    assert_eq!(image.pixel_color(3, 0).blue(), 0);
}
