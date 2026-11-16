# Test Plan

One row per scenario in the delta spec. Every row is automated.

## imaging/blur-gallery

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `blur_bokeh_no_rim` | Blur a dark (in-range) left half against a bright (out-of-range) right half with one pin, Light Bokeh 100 %, Bokeh Color 0, and compare to the same blur with Bokeh off. | The bokeh/lit ratio never exceeds the interior ratio by more than 5 %, so the blurred edge has no bright rim. | `pictura-filters::light_bokeh_does_not_rim_a_blurred_edge` |
| `blur_light_range_soft` | Apply Light Bokeh 100 % with Light Range 0..128 to a 0..255 luma ramp and measure adjacent output steps. | No step above 8 levels at the upper bound. | `pictura-filters::light_range_gate_is_continuous` |
