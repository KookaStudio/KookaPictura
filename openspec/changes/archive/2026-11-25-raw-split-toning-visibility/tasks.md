# Tasks

## 1. Tests first

- [x] 1.1 Add `pictura-adjust::split_toning_highlights_tint_the_upper_midtones` covering `rs_midtone_visibility`
- [x] 1.2 Add `pictura-adjust::split_toning_shadows_tint_the_lower_midtones` covering `rs_midtone_visibility`
- [x] 1.3 Tighten `tst_acr_dialog::splitToningHighlightsChangeThePreview` to a mid-grey layer and a measurable blue shift

## 2. Split toning render

- [x] 2.1 Overlap the shadow and highlight weights across the crossover
- [x] 2.2 Tint at full hue saturation, scaled by the Saturation slider
- [x] 2.3 Gamut-map by desaturating toward the neutral of the same luminance

## 3. Docs and verification

- [x] 3.1 Update the `codec/pictura-raw` split toning requirement and scenario
- [x] 3.2 Run the focused suites, fmt, clippy, file-size, guard, test-plan, and OpenSpec validation
- [x] 3.3 Archive the change with `openspec archive raw-split-toning-visibility`
