# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tsr_exact` | Tone dabs in parallel and on one thread, across tools, options, and tips | Same bytes and dirty rectangle | `pictura-paint::the_parallel_dab_equals_the_serial_raster` |
| `tsr_start` | Tone part of a layer lazily and from a whole-layer copy | Same bytes; unreached tiles stay unloaded | `pictura-paint::a_tone_stroke_loads_only_the_tiles_it_reaches` |
| `tsr_first_dab` | Tone a banded plane the history shares | Plane stays banded; far bands stay shared | `pictura-paint::a_tone_stroke_forks_only_touched_bands` |
| `tsr_stale_layer` | Change the layer bounds mid-stroke, then sample | No change reported | `pictura-paint::a_tone_stroke_refuses_a_layer_that_changed_under_it` |
| `tsr_write_back` | Stroke Dodge, Blur, and Smudge banded and serial, then cancel | Same bytes and dirty rectangle; cancel restores | `pictura-paint::per_dab_write_back_agrees_banded_and_serial` |
| `tsr_no_change` | Sponge across black pixels | No pixel, history, or change signal | `tst_retouch_tools::strokeChangingNothingRebuildsNothing` |
| Feel on the 16507×16196 world map | Drag Dodge, Burn, and Sponge at 500 and 1940 px | The brush keeps up with the pointer | manual: perceived lag needs a real display and pointer; `scripts/profile-tools.py` records the timings |
