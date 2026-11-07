# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Identity matrix is a no-op | Apply the RGB identity to a buffer | Buffer unchanged | `pictura-adjust::channel_mixer_known_example_and_identity` |
| Known blend | Red row `[50,100,0]` on `(50,100,200)` | Red 125, green/blue unchanged | `pictura-adjust::channel_mixer_known_example_and_identity` |
| `ia_cm_cmyk_identity` | Apply the 4×4 ink identity to unit ink samples | Every sample unchanged | `pictura-adjust::channel_mixer_cmyk_identity` |
| `ia_cm_cmyk_blend` | Cyan row `[0,100,0,0]`, input C 0.5, M 0.25 | Cyan 0.25, others unchanged | `pictura-adjust::channel_mixer_cmyk_blend` |
| `ia_cm_cmyk_constant` | Cyan constant -50 then +50 on C 0.25 | 0.75 then 0 | `pictura-adjust::channel_mixer_cmyk_constant` |
| `ia_cm_cmyk_invalid` | A CMYK weight or constant of ±201 | `InvalidParams`, samples unchanged | `pictura-adjust::channel_mixer_cmyk_rejects_out_of_range` |
| `alr_cm_cmyk_roundtrip` | Encode ink rows and decode | Ink rows and constants equal | `pictura-render::channel_mixer_ink_roundtrip` |
| `alr_cm_cmyk_k_source` | Hand-build a row with a non-zero fourth source | Decoded Black source equals it | `pictura-render::channel_mixer_ink_reads_k_source` |
| `alr_cm_cmyk_clamp` | Encode ink values above 200 / below -200 | Clamped, still decodes | `pictura-render::channel_mixer_ink_clamps` |
| `alr_cm_cmyk_composite` | Composite a non-identity CMYK mixer over a CMYK backdrop | Composite changes, stays in range | `pictura-render::composite_channel_mixer_cmyk` |
| `alr_cm_cmyk_destructive` | Apply a CMYK mixer destructively with a selection | Layer changes only inside coverage | `pictura-render::destructive_channel_mixer_cmyk` |
| `alr_cm_cmyk_native` | Apply a CMYK mixer to a depth-16 CMYK document | A sample's low byte differs from its high byte | `pictura-render::native_channel_mixer_cmyk_keeps_precision` |
| `alr_cm_cmyk_mono_rgb` | Apply a monochrome mixer to a CMYK document | R = G = B per pixel | `pictura-render::monochrome_channel_mixer_in_cmyk_document` |
| The channel-mixer kind becomes an adjustment layer | Add a `channel-mixer` layer | Layer carries `mixr` and reports as an adjustment | `pictura-app::channel_mixer_adjustment_layer` |
| The default Channel Mixer layer is neutral | Composite the default over a backdrop | Equals the backdrop | `pictura-render::default_channel_mixer_is_neutral` |
| The panel menu offers Channel Mixer | Build the Adjustments panel menu | Contains the Channel Mixer row | `tst_adjustments_panel::tilesAndMenuFollowCatalogue` |
| A non-neutral Channel Mixer layer changes the composite | Composite a non-identity row | Differs from the backdrop | `pictura-render::channel_mixer_layer_changes_non_uniform_backdrop` |
| `alr_cm_cmyk_default` | Add a `channel-mixer` layer in a CMYK document | Ink identity, backdrop unchanged | `pictura-app::channel_mixer_default_in_cmyk_document` |
| `nda_posterize_native` | Posterize levels 5 and 255 on a depth-16 store | Native levels; 255 unchanged | `pictura-adjust::native_posterize_16bit` |
| `nda_bw_cm_native` | Black & White / Channel Mixer on a depth-16 store | Not the `high * 257` widening | `pictura-adjust::native_bw_channel_mixer` |
| `nda_cm_cmyk_native` | CMYK mixer on a depth-16 CMYK document | Not the `high * 257` widening | `pictura-render::native_channel_mixer_cmyk_keeps_precision` |
| Edits keep unmodelled data and refuse invalid results | Run the existing `adjustment_params` edits | Reads back, clamps, refuses invalid Levels | `pictura-render::values_clamp_and_invalid_results_are_refused` |
| `ppe_cm_monochrome_toggle` | Toggle `monochrome` true then false on an identity RGB mixer | Gray group with 40/40/20, then three RGB groups with identity Green/Blue | `pictura-render::channel_mixer_monochrome_toggle` |
| `ppe_cm_cmyk_groups` | Build a CMYK Channel Mixer page | Four ink groups and four source keys per output | `pictura-render::channel_mixer_cmyk_page` |
| `ppe_cm_monochrome_hidden_cmyk` | Build a CMYK Channel Mixer page | No `monochrome` parameter | `pictura-render::channel_mixer_cmyk_page` |
| `ac_cm_monochrome` | Open the RGB dialog and click Monochrome | Output menu becomes Gray | `tst_image_adjustments::channelMixerMonochromeToggle` |
| `ac_cm_cmyk` | Open the dialog on a CMYK document | Four ink outputs and four source rows, no checkbox | `tst_image_adjustments::channelMixerCmykOutputs` |
| Photoshop-authored CMYK `mixr` parity | Open a CMYK PSD with a Channel Mixer layer and compare the outputs | Matches Photoshop | manual: no independent decoder or CS6 capture models the CMYK `mixr` layout; the layout is inferred and covered by round-trip/hand-built tests only |
