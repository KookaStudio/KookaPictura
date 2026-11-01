# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pane_blend_mode` | Preview on a Multiply layer | Multiplied colours; equals the commit at scale 1 | `pictura-app::pane_matches_the_commit_under_multiply`, `tst_shear_dialog::paneShowsTheBlendMode`, `tst_blur_gallery_dialog::canvasShowsTheBlendMode` |
| `pane_blend_if` | Preview on a layer with Blend If | Layer below shows; equals the commit at scale 1 | `pictura-app::pane_matches_the_commit_under_blend_if` |
| `pane_vector_mask` | Preview on a vector-masked layer | Hidden part shows the layer below, at the reduced position | `pictura-app::pane_places_a_vector_mask_at_reduced_scale` |
| `pane_selection` | Preview with a selection | Outside stays unfiltered | `pictura-app::pane_is_gated_by_the_selection` |
| `pane_open_preview` | Pane with a section preview open | Equals the pane with none open | `pictura-app::pane_ignores_an_open_preview` |
| `proxy_vector_mask` | Reduce whole and cropped | Edge at the scaled position | `pictura-render::reduced_vector_mask_keeps_its_place` |
| `proxy_vector_feather` | Reduce a feathered mask | Radius scaled, new identity | `pictura-render::reduced_vector_mask_scales_its_feather` |
| `proxy_full_scale` | Copy at scale 1 | Equals the document | `pictura-render::full_scale_proxy_keeps_a_masked_multiply_layer` |
| Other layers | Pane on a two-layer document | Only the active layer is filtered | `pictura-app::pane_filters_only_the_active_layer`, `tst_shear_dialog::paneKeepsTheOtherLayersUnsheared` |
| Nested layer | Pane on a layer inside a group | Resolves and filters | `pictura-app::pane_reaches_a_nested_layer` |
| Refused kind | Pane with an unknown kind, then with bad slots | No image, then the unfiltered document | `pictura-app::pane_refuses_an_unknown_kind` |
| Drag responsiveness | Drag Blur Gallery pins on a large layered document | No visible lag against master | manual: wall-clock feel on a real display and document |
| The workspace opens on the default band | Unchanged, carried by the modified requirement | As specified | `tst_tilt_shift::workspaceOpensOnTheDefaultBand` |
| The lines and dots reshape the band | Unchanged, carried by the modified requirement | As specified | `tst_tilt_shift::linesAndDotsReshapeTheBand` |
| Hovering shows the drag cursors | Unchanged, carried by the modified requirement | As specified | `tst_tilt_shift::hoverShowsTheDragCursors` |
| A drag previews a draft | Drag, then release | Draft while held, full proxy after | `tst_tilt_shift::aDragPreviewsADraftThenTheFullPicture` |
| The preview keeps the band sharp | Unchanged, carried by the modified requirement | As specified | `tst_tilt_shift::previewKeepsTheBandSharp` |
| Only the active layer blurs | Blur one of two layers | The other layer stays sharp | `tst_tilt_shift::onlyTheActiveLayerBlurs` |
| OK commits once | Unchanged, carried by the modified requirement | As specified | `tst_tilt_shift::menuCommitsOnce` |
