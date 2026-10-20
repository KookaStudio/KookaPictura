## 1. Engine seam

- [x] 1.1 Expose the per-layer ink box (`align::content_bounds`) as `pictura_render::layer_content_bounds`.
- [x] 1.2 Add `layer_content_bounds_extents` to the transform bridge and `transform_api::contentRect`.

## 2. Solver and drawing

- [x] 2.1 Add the pure `smart_guides.{h,cpp}` solver (snap offset, guide lines, gap).
- [x] 2.2 Add `ImageView` smart-guide state, magenta drawing, gap label, and test hooks.
- [x] 2.3 Register the new sources in `CMakeLists.txt`.

## 3. Tools

- [x] 3.1 Move tool: collect reference boxes at press, snap the raw delta, set/clear guides.
- [x] 3.2 Type tool: compute the moving ink box, snap the origin, set/clear guides.

## 4. Verification

- [x] 4.1 Add `tst_smart_guides` and register it in the test CMake list.
- [x] 4.2 `openspec validate --all --strict`.
- [x] 4.3 Build once and run `tst_smart_guides`, `tst_move_feedback`, and `tst_type_tools`.
- [x] 4.4 Run the guard, archive, test-plan, and openspec gates.
