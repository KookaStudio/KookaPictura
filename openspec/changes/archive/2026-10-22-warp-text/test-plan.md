# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `wt_menu` | Make a type layer current and dispatch `Type > Warp Text…` | The command is enabled and a `WarpTextDialog` opens for the layer | `tst_warp_text::menuOpensDialog` |
| `wt_menu_disabled` | Make a pixel layer current | The menu command and the options-bar Warp Text button are disabled | `tst_warp_text::menuDisabledForPixelLayer` |
| `wt_options_button` | Select the Horizontal Type tool with a type layer current; click Warp Text | A `WarpTextDialog` opens for the layer | `tst_warp_text::optionsButtonOpensDialog` |
| `wt_controls` | Open the dialog on an unwarped layer | Style lists None and the fifteen presets in CS6 order, None selected; the values range −100…100; orientation and values are disabled | `tst_warp_text::dialogControls` |
| `wt_default_bend` | Select Arc from None | Bend reads +50; orientation and values are enabled | `tst_warp_text::presetDefaultsBend` |
| `wt_reopen` | Commit Wave, Vertical, −30/10/−20; reopen | The dialog shows those values | `tst_warp_text::reopenShowsCurrentWarp` |
| `wt_preview` | Open the dialog and select Arc | The layer's pixels change and the history count does not | `tst_warp_text::changesPreviewLive` |
| `wt_cancel` | Preview Arc, then Cancel | The pixels and warp equal the pre-dialog layer and no state was recorded | `tst_warp_text::cancelRestoresLayer` |
| `wt_commit_undo` | Commit Arc, then undo | One `Warp Text` state was added; undo restores the unwarped pixels | `tst_warp_text::okRecordsOneState` |
| `wt_remove` | Commit Arc, then commit None | The layer reports no warp and its pixels equal the never-warped layer's | `tst_warp_text::noneRemovesWarp` |
| `wt_identity` | Render a spec with None, and with Arc at all-zero values | Both equal the unwarped render | `pictura-render::identity_warp_renders_unwarped` |
| `wt_arc` | Render one line with Arc +50 | The middle column's ink sits higher than the outer columns'; no ink falls outside the rect | `pictura-render::arc_bends_the_line` |
| `wt_axes` | Render Arc +50 Horizontal, Arc +50 Vertical, and Arc at Bend 0 with H Distortion 50 | All three differ from each other and from the unwarped render | `pictura-render::orientation_and_distortion_differ` |
| `wt_sharp` | Render Flag at Bend 20 | The share of partially covered ink pixels is within 1.5× the unwarped render's | `pictura-render::warped_edges_stay_sharp` |
| `wt_transform` | Render a 90°-rotated Arc layer | Its alpha equals the unrotated Arc alpha rotated 90°, within one pixel at the edges | `pictura-render::warp_precedes_transform` |
| `wt_live_edit` | Warp with Arc, then retype the text with the Type tool | The committed layer still reports Arc +50, and the edit preview was warped | `tst_warp_text::editKeepsWarp` |
| `wt_live_attrs` | Warp, change the size in the Character panel, then Free Transform scale | The warp is unchanged after each change | `tst_warp_text::attributesAndTransformKeepWarp` |
| `wt_psd_write` | Author a spec warped Fish, Vertical, 40/−10/25 | The descriptor holds `warpFish`, 40, −10, 25, `Vrtc` | `pictura-codec::authors_the_warp_descriptor` |
| `wt_psd_read` | Decode Arc H 50, `warpCustom`, and an empty descriptor | Arc 50 Horizontal, None, None | `pictura-codec::decodes_the_warp_descriptor` |
| `wt_roundtrip` | Add a warped type layer, write the PSD, read it back | `type_layer_spec` reports the same warp | `pictura-render::warp_survives_save_and_open` |
| `wt_refusal_rules` | Check plain, Faux Bold, on-path, area, and outline-stripped specs | Only the plain spec is warpable; each other spec reports its reason | `pictura-render::refusals` |
| `wt_faux_bold` | Invoke Warp Text on a Faux Bold layer | A warning names Faux Bold, no dialog opens, no state is recorded | `tst_warp_text::fauxBoldIsRefused` |
| `wt_on_path` | Invoke Warp Text on an on-path layer | A warning names the path, no dialog opens, no state is recorded | `tst_warp_text::typeOnPathIsRefused` |
| `wt_thumbnail` | Commit Arc, then commit None | The Layers row reports warped, then unwarped | `tst_warp_text::layersRowMarksWarp` |
| Warped thumbnail artwork | Warp a type layer and look at the Layers panel | The "T" card shows a bent baseline under the T, legible in light and dark themes | manual: the offscreen suite checks the row's warped role, not how the glyph reads |
| Visual match to CS6 | Warp the same word with each preset at ±50 and compare against CS6 references | The shapes match the CS6 preset family | manual: no CS6 oracle is available; the preset grids are a measured approximation |
| Warped PSD from Photoshop | Open a PSD saved by Photoshop with a warped type layer, edit its text, save, and reopen it in Photoshop | The warp survives in both applications | manual: no Photoshop-authored warped-type fixture is in the repository |
