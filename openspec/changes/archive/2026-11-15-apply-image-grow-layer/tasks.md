# Tasks

## Tests first

- [x] 1.1 `pictura-render::grown_layer_takes_the_source_across_the_document`
- [x] 1.2 `tst_apply_image::growsSmallLayerToDocument`
- [x] 1.3 `pictura-app::{apply_image_grows_a_smaller_layer_to_the_document,
      apply_image_preserve_transparency_keeps_the_rect,
      apply_image_transparency_lock_keeps_the_rect,
      apply_image_selection_confines_the_grown_blend,
      apply_image_preview_cancel_shrinks_the_grown_rect,
      apply_image_growth_previews_do_not_compound,
      apply_image_grown_native_store_keeps_precision,
      apply_image_grown_32_bit_store_keeps_values}`

## Implementation

- [x] 2.1 `pictura-app`: grow the Apply Image destination to the document in
      `apply_op_active_region` before the preview baseline is saved, when
      Preserve Transparency is off and the layer is not transparency-locked
- [x] 2.2 `pictura-render`: replace the `ponytail:` note on
      `apply_image_region` with the caller-grows contract
- [x] 2.3 `docs/dev/STATE.md` and `docs/dev/doc-code-parity-gaps.md` (SEL-1):
      record the grown destination and mark the gap shipped

## Verification

- [x] 3.1 `cargo nextest run -p pictura_app -p pictura-render`
- [x] 3.2 `openspec validate apply-image-grow-layer --strict`
- [x] 3.3 `ctest --test-dir build -R '^tst_apply_image' --output-on-failure`
- [x] 3.4 `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` (all 5713
      tests pass; the gate fails only on the pre-existing `shell-seams`
      findings in `cpp/menu_select.cpp`)
