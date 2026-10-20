# Tasks

## 1. Tests

- [x] 1.1 Add `tst_transform_skew::previewKeepsTheLayerColor` (plain raster preview at 83% zoom).
- [x] 1.2 Add `tst_transform_skew::styleKeepsTheLayerColor` (Color Overlay survives the preview).
- [x] 1.3 Add `tst_transform_skew::statusShowsTheSkewAction` (Free Transform → Skew → cancel).

## 2. Implementation

- [x] 2.1 Add `needs_rendered_appearance` / `rendered_layer_image` and use them in `build_transform_preview`.
- [x] 2.2 Add the status-bar `statusActionLabel` and its separator.
- [x] 2.3 Drive the label from `transform_session_mode` and connect `transformChanged`.

## 3. Verification

- [x] 3.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run -p pictura_app`.
- [x] 3.2 Build with CMake; run `tst_transform_skew`, the transform suites, and the self-test.
- [x] 3.3 `openspec validate transform-preview-status --strict`, then archive the change.
