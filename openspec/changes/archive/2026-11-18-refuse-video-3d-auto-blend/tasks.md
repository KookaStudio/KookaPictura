# Tasks

## 1. Tests first

- [x] 1.1 Add `auto_blend_refuses_video_and_3d_layers` to `crates/pictura-render/src/document_ops/layer_ops/auto_blend/tests.rs`: build a raster layer carrying a `PxSD` extra block, select it beside two rasters, and assert `can_auto_blend` is false, the plan is empty, and the document is unchanged.
- [x] 1.2 Add `pixel_source_block_marks_video_and_3d_layers` to `crates/pictura-codec/tests/layer_content.rs`: round-trip a `PxSD` layer through PSD and PSB, then assert `Layer::is_video_or_3d()` is true and the block bytes are preserved.

## 2. Detection and refusal

- [x] 2.1 Add `Layer::is_video_or_3d()` in `crates/pictura-core/src/layer_content/mod.rs`, documenting that `PxSD` is the shared 3D/video pixel-source block.
- [x] 2.2 Include `is_video_or_3d()` in Auto-Blend's `refused` predicate in `crates/pictura-render/src/document_ops/layer_ops/auto_blend/mod.rs`, and remove the stale `ponytail` note that says video/3D are not refused.

## 3. Verify

- [x] 3.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and the affected tests (`cargo nextest run -p pictura-core -p pictura-codec -p pictura-render` or the narrower test names).
- [x] 3.2 Run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`.
- [x] 3.3 Run `openspec archive refuse-video-3d-auto-blend` and confirm the change lands under `openspec/changes/archive/`.
- [ ] 3.4 Commit the code and `docs/dev/STATE.md` as `feat(layers): refuse video and 3D layers in Auto-Blend (#1110) TASK-ALLOWS-DOCS` and the archive move as `chore(openspec): archive refuse-video-3d-auto-blend (#1110)`; open the PR titled `feat(layers): refuse video and 3D layers in Auto-Blend Layers (#1110)` with the test plan's manual rows as a reviewer checklist.
