## 1. Render: decode `vscg` into typed fill content

- [x] 1.1 In `crates/pictura-render/src/composite.rs`, change `fn decode_solid_fill` to `pub(crate) fn decode_solid_fill` (no behavior change), so `fill.rs` can call it.
- [x] 1.2 In `crates/pictura-render/src/fill.rs`, add `pub(crate) fn decode_vector_fill(d: &[u8]) -> Option<Adjustment>`: slice off the leading 4-byte fill key, parse the rest with `pictura_codec::read_descriptor`, and dispatch on the descriptor content — a `Grad` item to `Adjustment::GradientFill(gradient_params_from_desc(&obj)?)`, a `Ptrn` item to `Adjustment::PatternFill(pattern_params_from_desc(&obj)?)`, a `Clr ` item to `crate::composite::decode_solid_fill(&d[4..])`; anything else (or a parse error) is `None`. Never panics. Add a `ponytail:` note that the key is consumed but the content is the dispatch authority (mirrors ag-psd `parseVectorContent`).
- [x] 1.3 Add `pub(crate) fn decode_layer_fill(layer: &Layer) -> Option<Adjustment>`: `layer.adjustment.as_ref().and_then(crate::composite::decode_adjustment).or_else(|| layer.extra_block(b"vscg").and_then(|b| decode_vector_fill(&b.data)))`. This is the single decision the render paths share (design D3).

## 2. Render: composite, effect coverage, and GPU fallback

- [x] 2.1 In `crates/pictura-render/src/composite.rs`, replace the `composite_layer` dispatch branch `else if let Some(data) = &layer.adjustment { if let Some(adjustment) = decode_adjustment(data) { … } }` with `else if let Some(adjustment) = crate::fill::decode_layer_fill(layer) { composite_adjustment(canvas, layer, doc, &adjustment); }`, and keep an adjustment layer with an undecodable block a no-op: only take the pixel/smart-source `else` when `layer.adjustment.is_none()` and the vector fill is absent. Confirm an undecodable adjustment is never treated as pixels.
- [x] 2.2 In `crates/pictura-render/src/fill.rs::fill_coverage_matte`, replace `layer.adjustment.as_ref().and_then(crate::decode_adjustment)` with `crate::fill::decode_layer_fill(layer)`, so layer effects are gated by a shape layer's vector fill.
- [x] 2.3 In `crates/pictura-render/src/gpu/mod.rs::check_supported`, after the adjustment check, return `GpuError::UnsupportedAdjustment` when `layer.extra_block(b"vscg").is_some()` on a visible layer, so the GPU falls back to the CPU (design D5). No shader.
- [x] 2.4 Confirm the `vscg` block stays in `Layer.extra_blocks` and is re-emitted verbatim: no codec, writer, or core-model change.

## 3. Fixture and oracles

- [x] 3.1 In `scripts/generate-fixtures.py`, add a `vector_fill()` builder registered as `"vector_fill.psd"` in `FIXTURES`: an 8×8 RGB document with a `Base` pixel layer and a document-sized `Shape` layer whose rect is `(0,0,8,8)` with `channel_info = []` and no channels. Attach a `vscg` `VectorStrokeContentSetting(key=b"SoCo", version=16, items={b"Clr ": Descriptor({b"Rd  ": Double(…), b"Grn ": Double(…), b"Bl  ": Double(…)}, classID=b"RGBC")})` under `Tag.VECTOR_STROKE_CONTENT_DATA`, and a closed `(1,1)-(5,5)` rectangle `VectorMaskSetting(version=3, flags=0, path=…)` under `Tag.VECTOR_MASK_SETTING1` using the existing `_closed_rect_path` recipe. The inner `Clr ` classID MUST be `RGBC` (psd-tools' own compositor rejects `null`).
- [x] 3.2 Run `python3 scripts/generate-fixtures.py`, commit `crates/pictura-codec/tests/fixtures/vector_fill.psd`, confirm a second run is byte-identical, and confirm no existing fixture file changed. Add a `vector_fill.psd` row to `crates/pictura-codec/tests/fixtures/README.md`.
- [x] 3.3 Add `crates/pictura-codec/tests/psd/vector_fill_oracle.rs` (new; `oracle.rs` is 1398/1400 lines): read `vector_fill.psd` with `psd-tools` via `python3 -c` (self-skipping when absent) and assert the shape layer's `vscg` key `SoCo`, version `16`, and `Clr ` components, that the layer kind is `shape`, and the composite colours inside/outside the `vmsk`. Add a Rust assertion that `read_psd` keeps the block in `extra_blocks` and that read→write→read preserves it and the whole `Document`.
- [x] 3.4 Append a `vscg` test to `crates/pictura-codec/tests/psd/agpsd_oracle.rs` (323 lines): run `node` + `ag-psd` over `vector_fill.psd`, read the shape layer's `vectorFill`, and assert it is a color fill with the authored components; self-skip when `node`/`ag-psd` is absent, mirroring the existing tests.

## 4. Render tests

- [x] 4.1 Add `crates/pictura-render/src/tests/vector_fill.rs` (new) and register `mod vector_fill;` in `crates/pictura-render/src/tests/mod.rs`. Do not extend `tests/adjustment.rs` (1396/1400 lines).
- [x] 4.2 Add a unit test that `decode_vector_fill` decodes a hand-built `[b"SoCo"][encode_solid_color_fill bytes]` block to the expected `SolidFill`, and a `[b"GdFl"][encode_gradient_fill bytes]` block to the expected `GradientFill`, proving the reuse of the existing decoders without a committed gradient fixture.
- [x] 4.3 Add a decode test that `read_psd` of `vector_fill.psd` leaves the `vscg` block in the `Shape` layer's `extra_blocks` and that `decode_layer_fill` yields the authored solid fill.
- [x] 4.4 Add a render test that composites the fixture (or an in-memory clone): assert the pixels inside the `(1,1)-(5,5)` rectangle carry the fill and the pixels outside it carry the base, proving the `vmsk` clip.
- [x] 4.5 Add tests that (a) a `vscg` fill with no vector mask covers the layer rect, (b) a decodable `SoCo` adjustment block takes precedence over a `vscg` block, and (c) a malformed `vscg` block leaves the composite equal to the document without it.
- [x] 4.6 Add a test that a document whose visible layer carries a `vscg` block reports the CPU backend (the GPU fallback), without requiring a Vulkan adapter.

## 5. Gates

- [x] 5.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`.
- [x] 5.2 `bash scripts/verify-full.sh` and a headless self-test (`./build/pictura --headless --self-test`); record counts. Confirm `vector_fill.psd` is byte-stable, the ag-psd oracle ran rather than self-skipped, and no existing golden changed.
- [x] 5.3 `openspec validate vector-fill-content --strict` and `openspec validate --all --strict`.
- [x] 5.4 Commit. No `docs/` change (a roadmap/`STATE.md` update stays a separate `TASK-ALLOWS-DOCS` commit). No `CMakeLists.txt` change and no self-test code, so no `ST_FAIL` code is consumed.

## 6. Explicitly not done (ceilings)

- [x] 6.1 The vector stroke (`vstk` `StrokeDescriptor` and its `strokeStyleContent`), `vogk`, `vsms`, noise gradients, multi-subpath boolean ops beyond union, text, and antialiasing stay unrendered; the raw `vstk`/`vogk`/`vsms` blocks stay preserved. Rasterizing a `vscg` shape layer and any authoring/editing/app UI are deferred. No new dependency.
