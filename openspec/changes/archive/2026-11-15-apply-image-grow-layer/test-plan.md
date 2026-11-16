# Test plan

One row per spec scenario. Coverage names an automated test, or `manual:` with
the reason it cannot be automated.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| ai_grow_document | Apply Image with Preserve Transparency off on a layer smaller than the document | rect becomes the document rect, old pixels stay put, the new area takes the source | `tst_apply_image::growsSmallLayerToDocument`, `pictura-app::apply_image_grows_a_smaller_layer_to_the_document`, `pictura-render::grown_layer_takes_the_source_across_the_document` |
| ai_grow_preserve | Apply Image with Preserve Transparency on for a smaller layer | rect and clear pixels are unchanged | `pictura-app::apply_image_preserve_transparency_keeps_the_rect` |
| ai_grow_locked | Apply Image with Preserve Transparency off on a transparency-locked layer | rect is unchanged | `pictura-app::apply_image_transparency_lock_keeps_the_rect` |
| ai_grow_selection | Active selection partly covers the document; Apply Image on a smaller layer | layer grows to the document, only selected pixels change, the rest stay transparent | `pictura-app::apply_image_selection_confines_the_grown_blend` |
| ai_grow_cancel | Preview a smaller layer, then cancel | rect and every channel are bit-identical to the original | `pictura-app::apply_image_preview_cancel_shrinks_the_grown_rect` |
| ai_grow_no_compound | Change the opacity twice on a smaller layer, then commit | equals one application at the final opacity to the original pixels | `pictura-app::apply_image_growth_previews_do_not_compound` |
| ai_grow_native | Apply Image to 16- and 32-bit layers with a native store, Preserve Transparency off | each store covers the document rect, keeps its precision, and the channels narrow it | `pictura-app::apply_image_grown_native_store_keeps_precision`, `pictura-app::apply_image_grown_32_bit_store_keeps_values` |
