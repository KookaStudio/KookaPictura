# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `abl_refused` | Ask availability in Lab, CMYK, Indexed and Bitmap documents, and with a Background, Smart Object or clipped layer selected, or one eligible layer | Each is unavailable and running the command leaves the document unchanged | `pictura-render::auto_blend_refuses_modes`, `pictura-render::auto_blend_refuses_selected_kinds`, `tst_auto_blend::refusedModes` |
| `abl_outside_selection` | Select two rasters beside a Smart Object | Available | `pictura-render::auto_blend_ignores_a_smart_object_outside_the_selection` |
| `abl_skipped` | Mixed selection of rasters, a group, an adjustment, a hidden layer, a type layer and an empty layer | Only the rasters gain masks | `pictura-render::auto_blend_skips_ineligible_layers` |
| `abl_menu` | Open Edit with one and with two raster layers selected | Disabled, then enabled | `tst_auto_blend::menuEnablement` |
| `abl_video_3d` | Build a raster layer carrying a `PxSD` block, select it beside two rasters, and ask for availability; also round-trip the block through PSD and PSB | Unavailable, plan empty, document unchanged; the block survives and `is_video_or_3d()` is true after loading | `pictura-render::auto_blend_refuses_video_and_3d_layers`, `pictura-codec::pixel_source_block_marks_video_and_3d_layers` |
| A Photoshop-authored 3D or video PSD | Open a real 3D or video PSD from Photoshop and check the layer carries `PxSD` | The layer is refused by Auto-Blend | manual: the repository has no Photoshop-authored 3D/video fixture; the codec test exercises the documented key instead |
