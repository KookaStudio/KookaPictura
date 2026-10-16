# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lmc_row_menu_enabled_mask` | Add a mask; open the row menu | Delete and Disable enabled; Add and Enable greyed | `tst_layers_mask_commands::rowMenuFollowsTheMaskState` |
| `lmc_row_menu_states` | Open the row menu with no mask, then with a disabled mask | Only Add; then Delete and Enable | `tst_layers_mask_commands::rowMenuFollowsTheMaskState` |
| `lmc_mask_button` | Click the mask button twice, then delete both masks | "Add layer mask" → "Add vector mask" → "Add a mask" (disabled) → "Add layer mask" | `tst_layers_mask_commands::maskButtonFollowsTheMasks` |
| `lmc_refine_mask` | Refine Mask with Feather 4 on a hard left-half mask | Dialog titled "Refine Mask" with Layer Mask output; the edge is partial; still linked; one "Refine Mask" state | `tst_layers_mask_commands::refineMask` |
| `lmc_refine_mask_any_layer` | Refine Mask on an unrasterized solid-colour fill layer's mask | Mask replaced; one "Refine Mask" state | `tst_layers_mask_commands::refineMaskOnAFillLayer` |
| `lmc_refine_preview_size` | Sample a 12×6 mask in a 16×10 document at 5×3 and 7×9; disable it | Equals the full-size coverage shrunk; a disabled mask reads its coverage | `pictura-app::the_mask_preview_samples_like_the_materialized_mask` |
| `lmc_rubylith_toggle` | Alt+Shift-click the mask thumbnail twice | Overlay on, then off; composite and history unchanged | `tst_layers_mask_commands::rubylithOverlay` |
| `lmc_rubylith_options` | Mask Options: blue, 80%; show the overlay | Defaults red/50%; a hidden pixel shows blue at 80% | `tst_layers_mask_commands::maskOptions`, `pictura-app::the_overlay_tints_hidden_coverage_at_the_opacity`, `pictura-app::the_overlay_is_capped_on_its_long_side`, `pictura-app::coverage_reads_the_rect_and_the_default` |
| `lmc_rubylith_follows_the_layer` | Show the overlay; Move Layer Down; show it again; select another layer | Off after the move; off after the selection; the canvas draws none | `tst_layers_mask_commands::rubylithEndsWithItsLayer` |
| `lmm_items` | Open the mask menu on a masked layer | CS6 rows in order, Refine Mask… and Mask Options… enabled | `tst_mask_edit::maskContextMenu`, `tst_layers_mask_commands::maskMenuRows` |
| `lmm_label` | Open the mask menu on an enabled, then a disabled mask | Disable, then Enable | `tst_mask_edit::maskContextMenu`, `tst_layers_mask_commands::maskMenuRows` |
| `lmm_delete` | Delete Layer Mask from the menu | Mask gone; pixels unchanged; one state | `tst_layers_mask_commands::maskMenuActions` |
| `lmm_apply` | Apply Layer Mask on a Hide All mask; undo | Alpha baked; mask gone; one "Apply Layer Mask" state | `tst_mask_edit::maskContextMenu`, `tst_layers_mask_commands::maskMenuActions` |
| `lmm_apply_smart` | Open the mask menu on a smart object | Apply disabled | `tst_mask_edit::maskContextMenu` |
| `lmm_apply_pixel_only` | Open the mask menu on a fill layer; run Apply anyway; apply on a group and a fill in the engine | Disabled; refused; mask kept; no state | `tst_layers_mask_commands::applyNeedsAPixelLayer`, `pictura-render::apply_refuses_a_group_and_a_fill_layer` |
| `lmm_selection` | Add Mask To Selection | The mask's white areas join the selection | `tst_mask_edit::maskContextMenu` |
| `lmm_refine_mask` | Pick Refine Mask… | Dialog titled "Refine Mask" on that mask | `tst_layers_mask_commands::refineMask` |
| `lmm_mask_options` | Pick Mask Options… | Display options with the current colour and opacity | `tst_layers_mask_commands::maskOptions` |
| Real right-click and tooltip hover | Right-click a mask thumbnail; hover the mask button in each state | The menu pops up at the pointer; the tooltip shows (also on the disabled button) | manual: offscreen QPA does not show popups or tooltips; the menu contents and tooltip text are automated above |
| Overlay at high zoom on a 16k document | Show the overlay, zoom to 400% | Overlay lines up with the mask; it is soft but not offset | manual: visual judgement; the 4096 px cap is a declared approximation (design.md) |
