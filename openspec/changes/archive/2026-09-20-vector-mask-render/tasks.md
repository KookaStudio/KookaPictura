## 1. Core: derived vector-mask view

- [x] 1.1 Add `crates/pictura-core/src/vector.rs` (new): `VectorMask { subpaths: Vec<VectorSubpath>, invert: bool, disabled: bool }`, `VectorSubpath { closed: bool, operation: i16, fill_rule: VectorFillRule, points: Vec<[i32; 2]> }`, and `VectorFillRule { EvenOdd, NonZero }`; all `Debug, Clone, PartialEq, Eq, Default`. `points` are document-pixel coordinates in 1/256-pixel units so the types stay `Eq` (design D2).
- [x] 1.2 Export the new types from `crates/pictura-core/src/lib.rs` and add `pub vector_mask: Option<VectorMask>` to `Layer`, defaulting to `None` in `Layer::default`.
- [x] 1.3 Update every exhaustive `Layer { … }` literal to add `vector_mask: None` (`crates/pictura-core`, `crates/pictura-codec`, `crates/pictura-render`, `crates/pictura-paint`, `crates/pictura-app` tests; about 32 sites). Do not change literals that use `..Default::default()`. `empty_layer` in `crates/pictura-codec/src/write.rs` needs the field only if it is exhaustive.

## 2. Codec: parse and flatten `vmsk`

- [x] 2.1 Add `crates/pictura-codec/src/vector_mask.rs` (new) with `pub(crate) fn resolve_vector_masks(layers: &mut [Layer], width: u32, height: u32)` that walks each layer (and its children), reads `extra_block(b"vmsk")`, and sets `layer.vector_mask`. Mirror the `smart_object::resolve_smart_objects` shape.
- [x] 2.2 Implement the block decode: require a big-endian `u32` version `== 3`, read the `u32` flags (bit 0 `invert`, bit 2 `disable`; bit 1 `not_link` ignored), then loop while at least 26 bytes remain, dispatching on the `u16` selector 0/3 (subpath length: `u16` count, `i16` operation, `u16` fill-rule field, skip 18), 1/2/4/5 (knot: six `i32`), 6 (skip 24), 7 (skip 24), 8 (`u16` + skip 22). Return `None` (leave the view unset) on an unknown selector, a knot without a preceding subpath, a subpath whose declared knot count exceeds the remaining records, or truncation; never panic.
- [x] 2.3 Scale each knot's six fixed-point values to document pixels in order preceding/anchor/leaving as `(y, x)`: `px = raw as f64 / 0x01000000 * width as f64`, `py = raw as f64 / 0x01000000 * height as f64`; store as 1/256-pixel `i32` (`(v * 256.0).round() as i32`).
- [x] 2.4 Flatten each cubic segment to a fixed 16-segment polyline with an `f64` De Casteljau evaluation; for a closed subpath also flatten the closing segment back to the first anchor. Store the polyline in `VectorSubpath.points` and keep `closed`, `operation`, and `fill_rule` (`NonZero` iff the fill-rule field is exactly `2`, else `EvenOdd`; design D3). Add a `ponytail:` comment naming the fixed subdivision and the adaptive upgrade.
- [x] 2.5 Wire `crate::vector_mask::resolve_vector_masks(&mut layers, width, height)` into `read_psd` right after `crate::smart_object::resolve_smart_objects` (`crates/pictura-codec/src/read.rs`), and add `mod vector_mask;` to `crates/pictura-codec/src/lib.rs`.
- [x] 2.6 Confirm `write_extra` needs no change: the raw `vmsk` block stays in `extra_blocks` and the derived view is not serialized, so read→write→read is unchanged.

## 3. Render: vector-mask coverage

- [x] 3.1 Add `crates/pictura-render/src/vector_mask.rs` (new) with `pub(crate) fn coverage(mask: Option<&VectorMask>, x: i32, y: i32) -> u8`: return `255` for `None` or `disabled`; collect the flattened points of every closed subpath; sample at the pixel centre (`(x + 0.5, y + 0.5)` in document pixels) with an even-odd crossing test, or a non-zero winding test when any subpath's `fill_rule` is `NonZero`; return `0` or `255` (hard edges), then `255 - c` when `invert`. Return `255` when there is no closed subpath. Add a `ponytail:` note for the fixed 0/255 coverage and per-pixel O(edges) scan (no active-edge table, no antialiasing).
- [x] 3.2 Add `mod vector_mask;` to `crates/pictura-render/src/lib.rs`.
- [x] 3.3 In `crates/pictura-render/src/composite.rs`, fold the vector coverage into `mask_alpha`: keep the existing raster-mask sample as a helper, then return `(raster as u16 * vector as u16 + 127) / 255`, so content and the layer-effect callers all pick it up (design D5). Keep the change small so `composite.rs` stays inside its 1200-line code cap (currently 1012).

## 4. GPU: include the vector mask in the mask plane

- [x] 4.1 In `crates/pictura-render/src/gpu/backend.rs`, extend `mask_has_data` to return true when the layer has `vector_mask.as_ref().is_some_and(|v| !v.disabled)`, so `assemble_mask` uses the per-pixel branch and uploads the CPU `mask_alpha` plane. No new GPU shader or rasterizer.
- [x] 4.2 Confirm the existing CPU/GPU parity tests still apply and, if there is an ignored GPU parity test, add a vector-mask case there (or leave the ignored test as the coverage and rely on the CPU render test). Do not add a non-ignored GPU test that needs a Vulkan adapter.

## 5. Fixture and oracles

- [x] 5.1 In `scripts/generate-fixtures.py`, add a `vector_mask()` builder and register `"vector_mask.psd": vector_mask` in `FIXTURES`. Author an 8×8 RGB document with, bottom-first, a `Base` pixel layer, a `Shape Inverted` solid-fill layer (`SoCo`, blue) carrying the closed `(0,0)-(6,6)` rectangle `vmsk` with flags bit 0 set, and a `Shape` solid-fill layer (`SoCo`, red) carrying the smaller closed `(1,1)-(3,3)` rectangle with flags 0. Attach the blocks with `psd_tools.psd.vector.VectorMaskSetting`/`Path`/`ClosedPath`/`PathFillRule`/`ClosedKnotLinked` via `Tag.VECTOR_MASK_SETTING1` on the layer record; use `Knot((y, x), …)` normalized coordinates.
- [x] 5.2 Regenerate the fixtures (`python3 scripts/generate-fixtures.py`), commit `crates/pictura-codec/tests/fixtures/vector_mask.psd`, and confirm the generator is byte-stable across a second run.
- [x] 5.3 Add `crates/pictura-codec/tests/psd/vector_mask_oracle.rs` (new): read `vector_mask.psd` with `psd-tools` (via `python3 -c`, self-skipping when absent) and assert each shape layer's `vector_mask` version 3, the authored flags, one closed subpath with the authored knots, and the pixel-space coverage of the authored rectangle. Do not extend `crates/pictura-codec/tests/oracle.rs` (1398/1400 lines).
- [x] 5.4 Add a `vmsk` test to `crates/pictura-codec/tests/psd/agpsd_oracle.rs` (260 lines): run `node` + `ag-psd` over `vector_mask.psd`, read `layer.vectorMask`, and assert the closed path's pixel knots and the `invert` flag; self-skip when `node`/`ag-psd` is absent, mirroring the existing tests.

## 6. Render tests

- [x] 6.1 Add `crates/pictura-render/src/tests/vector_mask.rs` (new) and register `mod vector_mask;` in `crates/pictura-render/src/tests/mod.rs`. Do not extend `tests/adjustment.rs` (1396/1400 lines).
- [x] 6.2 Add a decode test that `read_psd` of `vector_mask.psd` sets `vector_mask` on the `Shape` and `Shape Inverted` layers with the expected closed subpath and `invert` flags.
- [x] 6.3 Add a render test that composites the fixture (or an in-memory clone): assert the pixels inside the `Shape` rectangle `(1,1)-(3,3)` carry the red fill, the pixels inside the inverted layer's `(0,0)-(6,6)` rectangle but outside the `Shape` rectangle carry the base, and the pixels outside `(0,0)-(6,6)` carry the blue inverted fill.
- [x] 6.4 Add a malformed-block test building a `vmsk` payload with a bad version and one with an unknown selector, asserting `read_psd` succeeds with `vector_mask` unset (and does not panic).
- [x] 6.5 Add a unit test that `mask_alpha` combines a raster mask and a vector mask by multiplication and that a `disabled` vector mask changes nothing.

## 7. Gates

- [x] 7.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`.
- [x] 7.2 `bash scripts/verify-full.sh` and a headless self-test (`./build/pictura --headless --self-test`); record counts. Confirm the fixture is byte-stable and the ag-psd oracle ran rather than self-skipped.
- [x] 7.3 `openspec validate vector-mask-render --strict` and `openspec validate --all --strict`.
- [x] 7.4 Commit. No `docs/` change; a roadmap/`STATE.md` update stays a separate `TASK-ALLOWS-DOCS` commit.

## 8. Explicitly not done (ceilings)

- [x] 8.1 Open subpaths, `vscg`, `vsms`, subtract/intersect/xor geometry, feather, density, antialiasing, Reveal/Hide-All, and "Vector Mask Hides Effects" stay unrendered; the raw blocks stay preserved. No app UI, no self-test code, and no new dependency.
