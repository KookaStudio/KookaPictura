# Proposal: field-blur-gallery

## Why

`Filter ▸ Blur ▸ Field Blur` is a disabled stub in `cpp/command_tree.cpp`
(issue #340). CS6's Blur Gallery (`docs/06-filters/blur-gallery.md`, `FILT-092`)
builds a gradient of blurs from on-canvas pins, each with its own amount,
with the Blur Effects (Light Bokeh, Bokeh Color, Light Range) and — post-CS6,
on the owner's request — the Noise grain tab, and shares one workspace with
Iris Blur (#341) and Tilt-Shift (#342). The app has neither the variable-blur
engine nor the pin workspace, and the two follow-up issues reuse the shell
this change lands.

## What Changes

- `pictura-filters` gains `BlurPin { x, y, blur }`, `BokehParams`, and
  `BlurNoise` on `Filter::FieldBlur { pins, bokeh, noise }` in
  `blur/field.rs`: a per-pixel radius field from inverse-square-distance pin
  weights (pins may sit outside the image), a variable-radius blur built from
  a box-blur level pyramid whose two bracketing levels are linearly blended
  per pixel, then the Blur Effects pass (a luminance-proportional Light Bokeh
  lift, a Bokeh Color chroma push, both gated by the Light Range window) and
  the Noise pass (seeded Grain/Noise grain scaled by the field weight and
  damped in highlights). The kernel is deterministic, validates before
  mutating, leaves alpha to the render layer's alpha pass (which drops the
  colour-only effects), and joins the moving-kernel allowlist so an unlocked
  layer's transparency blurs too.
- `pictura-app` maps the `field-blur` kind from a variable slot list
  (`count, x, y, blur, …, bokeh …, noise …`, one pin plus the 13-slot tail
  canonical in the arity table) and routes the new
  `Filter ▸ Blur Gallery ▸ Field Blur…` row to a bespoke `BlurGalleryDialog`:
  a proxy preview with draggable circle pins, a Blur Tools panel with the
  Field Blur amount and the disabled Iris Blur / Tilt-Shift / Path Blur /
  Spin Blur rows, an Effects / Motion Effects / Noise tabbed panel (Bokeh
  and grain controls), `H` hide-UI, Preview / Cancel / OK, and one `"Filter"`
  history state on commit. The `Blur Gallery` submenu also lists disabled
  `Iris Blur…`, `Tilt-Shift…`, `Path Blur…`, and `Spin Blur…` stubs. Last
  Filter and Last Filter Settings reuse the stored slots.
- Verification: engine unit tests (field gradient, pin-outside-image
  anchoring, zero-pin no-op, validation, determinism, alpha, bokeh lift/tint/
  range, seeded grain and its controls), a kernel digest row, an oracle
  mapping row (no faithful ImageMagick operator, tolerance 0), bridge mapping
  tests, and a new Qt Test `tst_blur_gallery_dialog` (defaults/layout, the
  disabled rows, add/move/delete pins, live preview, the Effects and Noise
  tabs, commit/cancel, Last Filter).
- Documentation: `docs/06-filters/blur-gallery.md` records what shipped and
  its ceilings; `docs/dev/STATE.md` records the change.

## Capabilities

**New:** `imaging/blur-gallery` — the Blur Gallery workspace shell, the
Field Blur effect, and the Blur Effects and Noise post-passes.

**Modified:** `imaging/blur-filters` — the pin-interpolated variable-blur
kernel with its effects passes joins the blur family.

## Impact

- `crates/pictura-filters`: `blur/field.rs` (+ `blur/field/tests.rs`),
  `blur.rs`, `filter/types.rs`, `filter/apply.rs`, `lib.rs`,
  `tests/kernel_digests.rs`, `tests/oracle/mapping.rs`.
- `crates/pictura-render`: `filter.rs` (`filter_preserves_opacity`,
  `preview_apron`, the alpha pass's dropped effects).
- `crates/pictura-app`: `cxxqt_object/filter_map.rs`,
  `filter_map/arities.rs`, `filter_map_choices.rs`, `filter_map_tests.rs`,
  `cpp/filter_commands.cpp`, `cpp/frame_menus_filter.cpp`,
  `cpp/blur_gallery_dialog.{h,cpp}`, `cpp/blur_gallery_canvas.{h,cpp}`,
  `CMakeLists.txt`, `cpp/tests/tst_blur_gallery_dialog.cpp`,
  `cpp/tests/CMakeLists.txt`, `cpp/tests/tst_filter_menu.cpp`.
- No new dependency. CPU stays the oracle; no GPU kernel is added.
- Ceilings: the blur is a box-pyramid approximation of the gallery's closed
  disc/bokeh kernel, and the bokeh lift/tint curves and grain model are
  inferred approximations; the workspace previews a document proxy on its own
  canvas (not the main document canvas); the `M` mask view, the Iris
  Blur / Tilt-Shift / Path Blur / Spin Blur effects, and the Motion Effects
  tab's controls are not shipped.
