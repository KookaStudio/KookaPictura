# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tone16_dodge` | Run a Shadows Dodge stroke on a 16-bpc RGB layer whose native red is not 8-bit aligned. | The native red rises; the 8-bit red channel is `native >> 8`. | `pictura-paint::a_depth16_dodge_writes_the_native_store`, `tst_retouch_tools::sixteenBitDodgeLightsTheCanvas` |
| `tone16_burn` | Run a Shadows Burn stroke on the same 16-bpc layer. | The native samples fall; the 8-bit channels are their narrowing. | `pictura-paint::a_depth16_burn_writes_the_native_store` |
| `tone16_sponge` | Run a Sponge Saturate stroke on a 16-bpc colour patch. | The native colour spread rises; the 8-bit channels are their narrowing. | `pictura-paint::a_depth16_sponge_writes_the_native_store` |
| `tone16_gray` | Run Dodge and Sponge on a 16-bpc Grayscale layer with one native plane. | The grey plane moves; the store still has only channel 0. | `pictura-paint::a_depth16_grayscale_tone_uses_the_single_plane` |
| `tone16_precision` | Run the same stroke on a 16-bpc layer and an 8-bit layer seeded with the narrowed samples. | The native result differs from the widened 8-bit result. | `pictura-paint::a_depth16_tone_is_not_the_8bit_widening` |
| `tone16_cancel` | Begin a 16-bpc tone stroke, paint one dab, cancel. | The native plane equals its pre-stroke samples. | `pictura-paint::cancelling_a_depth16_tone_restores_the_native_store` |
| `tone16_32bpc_refused` | Begin a tone stroke on an 8-bit working document that retains a 32-bit source store. | `Stroke::begin_tone` returns `UnsupportedDepth`; no pixel changes. | `pictura-paint::a_retained_32bit_store_refuses_toning`, `tst_retouch_tools::thirtyTwoBitDisablesToneTools` |
| `tone16_8bit_unchanged` | Run the existing 8-bit tone engine tests. | They pass with the same expected bytes. | `pictura-paint::the_parallel_dab_equals_the_serial_raster`, `pictura-paint::grayscale_dodge_and_burn_use_the_gray_plane`, `pictura-paint::the_sponge_drains_and_lifts_colour_but_not_grey` |
