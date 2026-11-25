# Test plan: type-move-responsiveness

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `mvp_type_ink` | Move a paragraph type layer whose box is large but not near the canvas. | The moving crop and the damage are the ink, not the box. | `pictura-app::a_type_move_previews_and_damages_its_ink_not_the_whole_box` |
| `mvp_type_effects` | Move a type layer carrying an enabled drop shadow. | The reach is the ink plus the shadow pad, smaller than the paragraph box. | `pictura-render::type_ink_reach_keeps_the_shadow_pad` |
| `mvp_type_source` | Move a type layer with no raster channels. | The press materializes the type raster and the move crop is its ink. | `pictura-app::a_channel_less_type_layer_materializes_for_a_move` |
| `mvp_type_clipped` | Move a type layer clipped to a stationary base. | The move crop is the type's ink, not the base's reach. | `pictura-app::a_clipped_type_layer_move_trims_to_its_ink` |
