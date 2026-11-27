use super::{layer_style_value, set_layer_style_value};

#[test]
fn advanced_blending_flag_values() {
    let mut layer = pictura_codec::layer_from_parts(pictura_core::LayerParts::default());
    let keys = [
        "blending.transparencyShapes",
        "blending.layerMaskHidesEffects",
        "blending.vectorMaskHidesEffects",
    ];
    for (key, initial) in keys.into_iter().zip([1.0, 0.0, 0.0]) {
        assert_eq!(layer_style_value(&layer, key), Some(initial));
        assert!(set_layer_style_value(&mut layer, key, 1.0 - initial));
        assert_eq!(layer_style_value(&layer, key), Some(1.0 - initial));
        assert!(!set_layer_style_value(&mut layer, key, 1.0 - initial));
        assert_eq!(
            [
                layer.transparency_shapes,
                layer.layer_mask_hides_effects,
                layer.vector_mask_hides_effects,
            ],
            match key {
                "blending.transparencyShapes" => [false, false, false],
                "blending.layerMaskHidesEffects" => [false, true, false],
                _ => [false, true, true],
            }
        );
    }
}
