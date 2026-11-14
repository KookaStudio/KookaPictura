# Tasks

## 1. Engine helpers

- [x] 1.1 Add `fit_smart_object_to_canvas(doc, path)` to `pictura-render`: source-native rect from the layer, uniform `min(1, canvas/source)` scale, centred fitted rect, no upscale, no payload mutation; re-export it.
- [x] 1.2 Add `can_transform_smart_object_rect(doc, path)` and `transform_smart_object_rect(doc, path, rect)` to `pictura-render`: eligible channel-less embedded smart object, positive destination rect, payload and preserved blocks unchanged.
- [x] 1.3 Add Rust tests for landscape/portrait fit, no-upscale, payload retention, and the smart-rect refusal/preservation cases in `crates/pictura-render/src/tests/smart_object_actions.rs`.

## 2. Place bridges

- [x] 2.1 In `impl_layers_smart_object.rs`, fit the PSD/PSB place inside the same `"Place"` edit before selecting the layer.
- [x] 2.2 In the same bridge, clear the raster proxy after `convert_to_smart_object` and fit the raster place inside the same edit, so it renders from the native embedded source.
- [x] 2.3 Confirm the existing `tst_place_sequence` Place tests still target the unchanged sequencing/cancel behavior; the new fit cases live in `tst_place_fit`.

## 3. Free Transform scale/translation

- [x] 3.1 In `impl_transform/session.rs`, detect the single-member, angle-zero, skew-zero, positive-scale, non-projective, non-warp target that `can_transform_smart_object_rect` accepts; compute the axis-aligned destination rect from the session quad and commit through `transform_smart_object_rect`, recording one `"Free Transform"` state and bypassing the deferred pixel path.
- [x] 3.2 Leave rotation, skew, warp, negative scales, masks, linked sets, and other targets on the existing rasterizing path.

## 4. Tests and UI suite

- [x] 4.1 Add `crates/pictura-app/cpp/tests/tst_place_fit.cpp` with PSD fit/session start, no-upscale, scale-up preserving the smart object and native source, raster fit, and Place-state cases.
- [x] 4.2 Register `tst_place_fit` in `crates/pictura-app/cpp/tests/CMakeLists.txt`.

## 5. Documentation and state

- [x] 5.1 Update `docs/05-layers/smart-objects.md`, `docs/10-workflow-io/open-and-new.md`, and `docs/03-tools/move-and-transform.md` for the fit/non-destructive scale scope and the rotation/skew/warp limitation.
- [x] 5.2 Update `docs/dev/STATE.md` and `ROADMAP.md` after the user verifies the change.

## 6. Verification and archive (deferred until the user prompts)

- [x] 6.1 Run the targeted build/tests and `openspec validate --all --strict`; fix failures.
- [x] 6.2 Run `openspec archive place-fit-to-canvas` after the user's verification.
