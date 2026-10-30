# Test Plan

One row per scenario in `specs/ui/color-picker-drag/spec.md`. The build
counter is `#[cfg(test)]`, so the reuse and rebuild checks are unit tests in
`color_management.rs`. nextest runs each in its own process, so no other test
moves the counter.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cpd_reuse` | Call `picker_in_gamut`, `picker_closest_rgb`, `picker_cmyk` and `picker_rgb` once, read the build counter, then call each again for a sweep of colours | The counter does not move after the first call | `pictura-app::picker_services_reuse_their_transforms` |
| `cpd_rebuild` | Read the build counter, install the Coated profile's ICC bytes through `install_picker_profile`, then run the services again | The counter rises by exactly one, and the services return the same values as before the install | `pictura-app::installing_a_profile_rebuilds_the_picker_pair` |
| `cpd_exact` | Convert a grid of sRGB colours to Coated CMYK and back, once through one `SpaceTransform` reused for every colour and once through `convert_space` per colour; then compare the picker services against the one-shot codec path | Every output byte is identical | `pictura-color::space_transform_matches_convert_space`, `pictura-app::picker_services_reuse_their_transforms` |
| `cpd_drag` | Open a `ColorPickerDialog`, press in the colour field, send 200 left-button moves across it, and read the hex field after each | The hex readout matches the field colour under the cursor after every move, and the drag finishes within the budget | `tst_color_picker::fieldDragUpdatesEverySample` |

## Manual checks

- Reproduce #1037: Layer > New Fill Layer > Solid Color, accept the New Layer
  dialog, and drag through the colour field and the hue ramp. The marker and the
  readouts follow the cursor as smoothly as in the Color panel. Visual
  smoothness on a real display cannot be asserted offscreen; the automated drag
  check covers the per-sample cost.
