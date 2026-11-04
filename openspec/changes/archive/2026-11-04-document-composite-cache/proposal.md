# Proposal

## Why

`pictura-render` only produces buffers; `pictura-app` owns the document composite cache. The app decides whether the cached composite is shape-valid, stores it, and patches it, so the "a patched region equals the same slice of a full composite" invariant crosses a crate seam and is only testable inside `pictura-app` (there are Rust `#[test]` oracles at `crates/pictura-app/src/cxxqt_object/tests.rs:870` and `crates/pictura-app/src/cxxqt_object/canvas_view_test.rs:388`). Moving the cache operations behind the render seam makes the invariant a render unit test and gives every cache writer one cache-write seam.

## What Changes

- Add `pictura_render::DocumentComposite`, a borrowing facade over `Document` that owns the composite cache operations: full store, region patch, refresh, and shape validity.
- Add a render-level scope (`Full` or a list of document regions) and a `Refreshed` result carrying the raw straight-alpha rendered buffer, the clamped document rectangle, and the backend that ran.
- Move `store_composite`, `patch_composite_region`, `patch_buffer_region`, and the `rgba_frame` plane-expansion helper from `pictura-app` into `pictura-render`; delete the app copies rather than wrapping them.
- Route the edit transaction and every other cache writer (open, filter and image-adjust jobs, paint commit/cancel, history snapshot) through the new type.
- Keep `Document.composite` as the cache storage and the PSD merged-composite save data; do not change the codec, history plane tracking, or `document_ops`.
- Move the buffer mode, channel-count, and dimension checks into render as `DocumentComposite::is_shape_valid`; keep the app's live-state gates (live stroke, lifted transform, display preview, native depth) in `picture::is_current`.
- `refresh(Full)` on a layerless document does not composite the empty stack; it stores the embedded composite through the same store rule, so an RGB 3-channel embedded composite becomes 4-channel exactly as before, and a non-RGB composite keeps its plane count. A layerless `Regions` refresh is unreachable (no layer edit) and is a no-op.
- Add a render differential oracle: for deterministic fixtures, patch a clone of the full composite with a region composite and assert byte equality, including a banded-reference pin and a grayscale channel-count case.
- No externally observable behavior change and no new dependency.

## Capabilities

No capability delta: this is a pure refactor with no externally observable behavior change (`skip_specs: true` in `.openspec.yaml`). The current-picture, described-damage, and banded-compositing behavior is unchanged; only the crate that owns the cache operations moves.

## Impact

- `crates/pictura-render/src/document_composite.rs` (new) and `crates/pictura-render/src/lib.rs`.
- `crates/pictura-app/src/cxxqt_object/helpers_composite.rs`: `store_composite`, `patch_composite_region`, `patch_buffer_region`, and `rgba_frame` are removed; `into_rgba_frame` stays.
- `crates/pictura-app/src/cxxqt_object/edit.rs`, `impl_core.rs`, `impl_core/display.rs`, `impl_paint.rs`, `picture.rs`, `open_jobs/imp.rs`, `filter_tools/jobs/imp.rs`, `filter_tools/imp.rs`, `image_adjust/jobs/imp.rs`, `impl_history.rs`, `impl_transform/mod.rs`, `display_preview.rs`, `clipboard/imp.rs`: cache writers and importers move to the render type.
- App test modules that import the helpers are repointed: `tests.rs`, `tests_impl.rs`, `canvas_view_test.rs`, `filter_tools/jobs/imp/tests.rs`.
- App-side oracle tests that only duplicate the region-equals-full check move to `pictura-render`; display/level-0 integration tests stay.
- No codec, history-plane, or `document_ops` changes; `Document.composite` stays in `pictura-core`.
