# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tone_airbrush_off` | With Dodge active and Airbrush off, press and hold still for several held-dab intervals, then release. | The pixel stops changing after the initial dab; one history state. | `tst_retouch_tools::airbrushOffStaysStill` |
| `tone_airbrush_on` | With Dodge active and Airbrush on, press and hold still, then release. | The pixel keeps lightening while held; release records one "Dodge" state. | `tst_retouch_tools::airbrushBuildsWhileHeld` |
| `tone_airbrush_shortcut` | With Dodge active, press `Shift+Alt+P`, then again. | The model flag flips and `optionsDodgeAirbrush` follows both ways. | `tst_retouch_tools::shiftAltPTogglesToneAirbrush` |
| `tone_range_quick_keys` | With Dodge active, press `Shift+Alt+S`, `Shift+Alt+M`, `Shift+Alt+H`. | Range becomes 0, 1, 2 and `optionsDodgeRange` follows. | `tst_retouch_tools::rangeQuickKeys` |
| `tone_sponge_quick_keys` | With Sponge active, press `Shift+Alt+S`, `Shift+Alt+D`. | Mode becomes Saturate, then Desaturate, and `optionsSpongeMode` follows. | `tst_retouch_tools::spongeQuickKeys` |
| `tone_quick_key_ignored` | With the Brush active, press `Shift+Alt+S`, `Shift+Alt+M`, `Shift+Alt+H`, `Shift+Alt+D`. | Every tone option group is unchanged. | `tst_retouch_tools::quickKeyIgnoredForOtherTools` |
| Draining and lifting colour | Drag the Sponge across a red at the defaults, then again in Saturate mode. | Each drag records one "Sponge" state; the first drains colour and the second lifts it. | `tst_retouch_tools::spongeTool` |
| `tone_grayscale_sponge` | Convert a dark/light grey seed to Grayscale, then Sponge Saturate across both bands. | The dark band darkens and the light band lightens; one history state. | `tst_retouch_tools::grayscaleSpongeChangesContrast`, `pictura-paint::grayscale_sponge_moves_contrast_around_middle_grey` |
| `tone_grayscale_dodge_burn` | Dodge then Burn a mid-grey Grayscale document. | Dodge raises the grey value, Burn lowers it, and the unused planes stay untouched. | `pictura-paint::grayscale_dodge_and_burn_use_the_gray_plane` |
| `sponge_grayscale_contrast` | Same as `tone_grayscale_sponge`. | Same expected result. | `tst_retouch_tools::grayscaleSpongeChangesContrast`, `pictura-paint::grayscale_sponge_moves_contrast_around_middle_grey` |
| `tone_32bpc_disabled` | Open an 8-bit document, convert it to 32 bpc, refresh, then inspect and try to activate the tone tools. | The Dodge/Burn/Sponge flyout rows are disabled; activating one leaves Move active. | `tst_retouch_tools::thirtyTwoBitDisablesToneTools` |
| `tone_32bpc_fallback` | Activate Dodge on an 8-bit document, convert it to 32 bpc, refresh. | The active tool becomes Move. | `tst_retouch_tools::thirtyTwoBitFallsBackToMove` |
| `tone_32bpc_refused` | Begin a tone stroke on a 32-bpc document directly in the engine. | `Stroke::begin_tone` returns `UnsupportedDepth`; no stroke/history is created. | `pictura-paint::thirty_two_bit_documents_refuse_toning` |
