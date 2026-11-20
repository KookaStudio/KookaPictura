# Test Plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `tob_xy_slider_span` | Begin a Free Transform, inspect the X/Y popup sliders, move the X slider one step, then type X = -50000. | The sliders span ±10,000 with a 100 px single step; one step moves X by 100 px; the typed value still moves the target to -50000. | `tst_transform_options::xySliderStepsAboutHundredPixels` |
