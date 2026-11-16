# Tasks: svg-export-vector-primitives

## 0. Decisions and research

- [x] 0.1 Record the CC vector-sample research result: no sample exists in the
  repository, so the even-odd path rule and the CSS blend map stay inferred
  (D6).
- [x] 0.2 Confirm the codec paint-server split: geometry is computed in
  `pictura-render` and serialized by `pictura-codec` (D1).
- [x] 0.3 Confirm the native subset: linear/radial gradients and solid strokes
  are vector; angle/reflected/diamond gradients, gradient/pattern strokes, and
  other active effects rasterize (D7).

## 1. Codec

- [x] 1.1 Replace the shape's solid fill with `SvgPaint` and add
  `SvgGradient`, `SvgGradientStop`, `SvgPattern`, and `SvgStroke`.
- [x] 1.2 Emit `<linearGradient>` / `<radialGradient>` / `<pattern>` in
  `<defs>` with document-unique ids and the fill reference.
- [x] 1.3 Emit center, inside, and outside stroke attributes, including the
  inside/outside masks.
- [x] 1.4 Add codec unit tests for each paint server and stroke position.

## 2. Render

- [x] 2.1 Map `GradientFillParams` and the layer rect to the codec gradient
  geometry, including reverse stops and the degenerate fallbacks.
- [x] 2.2 Map `PatternFillParams` and the document patterns to a
  nearest-neighbour scaled tile and user-space origin.
- [x] 2.3 Decode the layer effects and emit a stroke only when the solid stroke
  is the sole active effect.
- [x] 2.4 Keep the raster fallback for unsupported gradients, strokes, and
  other effects; add render unit tests.

## 3. Bridge and dialog

- [x] 3.1 Add `svg_from_layer(view, index)` to the bridge, cloning the document
  with that top-level layer alone.
- [x] 3.2 Make `exportAsFromView` write one SVG per checked row, prompting for
  a file or folder as the row count requires.
- [x] 3.3 Add the per-layer Qt Test and the multi-row manual checklist entry.

## 4. Oracle

- [x] 4.1 Extend `svg_oracle.rs` with gradient, pattern, and outside-stroke
  content; keep the self-skip.

## 5. Docs

- [x] 5.1 Update `docs/10-workflow-io/export-formats.md` (`TASK-ALLOWS-DOCS`).
- [x] 5.2 Update `docs/dev/STATE.md` (`TASK-ALLOWS-DOCS`).

## 6. Verification

- [x] 6.1 `cargo fmt --all` and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 6.2 `cargo nextest run -p pictura-codec -p pictura-render` and the Qt
  `tst_export_as` suite.
- [x] 6.3 `openspec validate --all --strict` and `scripts/check-test-plan.py`.
- [ ] 6.4 `bash scripts/verify-fast.sh` — blocked by the pre-existing
  `pictura-render::oracle::imagemagick_fixtures_reproduce` (`ramp_Divide.rgba`)
  fixture mismatch, unrelated to this change; every other gate passes.
