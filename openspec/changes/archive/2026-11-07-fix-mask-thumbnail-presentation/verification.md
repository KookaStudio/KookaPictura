# Verification

The production reproduction opened a generated 120×400 PSD with uniform mask
coverage 128. Its opaque 7×24 preview was fitted into a 10×32 card, leaving the
rightmost column as alternating checkerboard values 200/255. The fixed app
capture has coverage 128 next to a solid black outline instead.

The cached mask-target rectangle was independent of the selection-aware brackets.
Removing that rectangle leaves the normal black outline and active-thumbnail
brackets. Group masks retain brackets while their folder icons remain unbracketed.

- `scripts/verify-full.sh`: passed; 8,030 passed, 124 skipped, zero failed.
- All 5,071 workspace Rust checks passed; the complete Qt layer passed 2,452
  checks with its two existing skips.
- Focused mask and Layers suites: all eight CTest entries passed.
- Four new Qt cases cover opaque imported cards at 16/24/40 pixels and display
  scales 1/1.5/2, black outlines, pixel/group brackets, raster/layer transitions,
  and retained mask/vector modifier behavior.
- Strict OpenSpec validation, test-plan checks, formatting, Clippy, guards,
  control checks and memory-harness checks passed.

All 19 scenario rows are automated. No manual reviewer checks, dependency changes
or golden baseline updates were needed. The missing protection was checking the
rendered card, including its edges, rather than only the opaque source image.
