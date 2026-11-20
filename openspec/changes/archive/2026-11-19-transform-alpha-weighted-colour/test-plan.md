# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Erased edges keep their colour after a scale | Build a layer: opaque yellow left half, erased (alpha 0, black) right half and an erased hole; scale 250% with `Bilinear` and with `Bicubic`. | Every output pixel with alpha > 0 has R, G, B within 1 of the yellow. | `pictura-render::a_scaled_erased_edge_keeps_its_colour` |
| The layer edge does not darken | Rotate an opaque yellow layer 30° with `Bilinear`. | Every pixel with 0 < alpha has the yellow colour within 1. | `pictura-render::a_rotated_edge_keeps_its_colour` |
| Opaque content is unchanged | Sample a ramp with an all-255 alpha through `sample_over_alpha` and `sample` at fractional and integer positions, every method. | Identical samples. | `pictura-render::weighted_equals_plain_on_opaque_taps` |
| The native store is weighted too | Give the erased-edge layer a 16-bit store and scale it 250% with `Bilinear`. | Native colour samples with alpha > 0 stay within 1/255 of yellow; the 8-bit channel equals the store's narrowing. | `pictura-render::a_scaled_native_store_keeps_its_colour` |
| An erased hole stays erased after a scale | Give an opaque yellow layer a 16-bit store, then erase a 12×12 hole in the 8-bit planes only; scale 250% with `Bilinear`. | At least 400 output pixels have alpha 0; each store plane's narrowing equals its 8-bit channel. | `pictura-render::a_scale_after_an_erase_keeps_the_hole` |
| An erased hole stays erased after a warp | The same layer, warped onto a quad twice its size with `Bilinear`. | At least 400 output pixels have alpha 0; each store plane's narrowing equals its 8-bit channel. | `pictura-render::a_warp_after_an_erase_keeps_the_hole` |
| A half-covered pixel reads at half luma | Build a 4×1 grey layer with alphas 255, 128, 0, 255 and read its work image. | Luma is full, ×128/255, 0, full; coverage is true, true, false, true. | `pictura-render::work_luma_is_premultiplied_by_alpha` |
| A perspective-transformed layer still aligns | Free Transform the moving copy onto a trapezoid, plan Perspective. | A homography that maps the trapezoid corners back to the reference corners. | `pictura-render::perspective_recovers_a_homography` |
