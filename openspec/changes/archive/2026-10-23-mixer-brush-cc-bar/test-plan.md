# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mx_mode_multiply` | Mixer dab at Multiply over an opaque base | Channel-wise product of deposit and base | `pictura-paint::mixer_applies_mode_opacity` |
| `mx_opacity_cap` | Mixer dab at Opacity 50 % over an opaque base | Half-way colour | `pictura-paint::mixer_applies_mode_opacity` |
| `mx_bar_cc` | Activate the Mixer Brush; look for Mode, Opacity and Sample All Layers; tick the box | No Mode or Opacity; the box is enabled and writes `sampleAllLayers` | `tst_options_bars_parity::mixerBarMatchesCc` |
| `mx_sample_all` | Wet dab on a white layer with a blue sampled composite, then without it | Blue is deposited and carried only with the composite | `pictura-paint::sample_all_layers_picks_up_from_the_composite` |
| `mx_preset_order` | Stroke a red brush from a blue band into white at Dry, Moist, Very Wet | Dry pure red, Moist carries blue, Very Wet smears blue | `pictura-paint::preset_strokes_hold_their_documented_ordering` |
| Manual: Sample All Layers in the app | Paint with the Mixer Brush over a layer stack with the box on and off | With it on, colour is picked up from layers beneath the active one | manual: needs a real canvas and eyes on the smear |
