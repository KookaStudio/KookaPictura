# Tasks: field-blur-gallery

## 1. Engine kernel

- [x] 1.1 Add `BlurPin`, `BokehParams`, `BlurNoise`, `MAX_FIELD_RADIUS`,
  `MAX_FIELD_PINS`, and `blur/field.rs::field_blur` (inverse-square-distance
  radius field, box-blur level pyramid, bracketing-level blend, then the
  bokeh lift/tint and seeded grain passes), exported from the crate root.
  Verify: field gradient, outside-pin anchoring, zero-pin no-op, validation,
  determinism, alpha, tiny-image, bokeh lift/tint/range, and grain unit tests
  pass.
- [x] 1.2 Add `Filter::FieldBlur { pins, bokeh, noise }`, dispatch it in
  `filter/apply.rs`, add it to `filter_preserves_opacity`'s moving list and
  to `preview_apron`, and drop the colour-only effects on the alpha pass.
  Verify: `cargo nextest run -p pictura-render` and the filters tests pass.
- [x] 1.3 Add the `"field blur"` kernel digest row and the `FieldBlur`
  no-equivalent oracle row. Verify: `kernel_digests` and the oracle mapping
  tests pass.

## 2. App

- [x] 2.1 Map the `field-blur` kind from the variable pin list plus the
  13-slot effects tail (`FIELD_BLUR_SLOTS = 17` canonical) and register the
  arity. Verify: `filter_map_tests` (defaults, canonical list, one- and
  two-pin tails, malformed refusal, 16-pin cap) pass.
- [x] 2.2 Add `blur_gallery_canvas.{h,cpp}` (draggable circle pins,
  hit-testing, move/blur drags, add/delete, `H`) and
  `blur_gallery_dialog.{h,cpp}` (proxy preview, Blur Tools rows, the
  Effects/Motion Effects/Noise tabs with the bokeh and grain controls,
  preview/cancel/OK, slot round-trip) and list them in `CMakeLists.txt`.
- [x] 2.3 Add the `Filter ▸ Blur Gallery` submenu (Field Blur…, Iris Blur…,
  Tilt-Shift…, Path Blur…, Spin Blur…; only Field Blur enabled) in
  `command_tree.cpp`, the `field-blur` row to `filter_commands.cpp`, and
  route the kind to `BlurGalleryDialog` in `frame_menus_filter.cpp`. Verify:
  the menu row is enabled and the table-wide filter-menu tests pass.
- [x] 2.4 Add `cpp/tests/tst_blur_gallery_dialog.cpp` (layout/defaults, the
  disabled rows, add/move/delete pins, live preview, the Effects and Noise
  tabs, commit/cancel, Last Filter) and register it in `tests/CMakeLists.txt`;
  extend `tst_filter_menu`'s slot cases. Verify:
  `ctest --test-dir build -R '^tst_' --output-on-failure` is green.

## 3. Documentation

- [x] 3.1 Update `docs/06-filters/blur-gallery.md` (shipped state, ceilings)
  and `docs/dev/STATE.md`. Verify: `openspec validate --all --strict`.

## 4. Integration verification

- [x] 4.1 Run `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` and the Qt
  suite; fix any regression.
