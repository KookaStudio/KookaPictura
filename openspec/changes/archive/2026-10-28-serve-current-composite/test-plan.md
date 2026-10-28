# Test plan — serve-current-composite

One row per scenario in `specs/compositing/current-picture/spec.md`. Coverage
names the test that proves the row; `manual:` is used only where the check
cannot run in CI.

Tests named here are created by `tasks.md`; `scripts/check-test-plan.py` reports
"no such test" until the implementation tasks land them.

Run rows with:

```bash
cargo nextest run -p pictura_app picture
cargo nextest run -p pictura_app composites_nothing
cargo nextest run -p pictura_app selection
cargo nextest run -p pictura_app fill
ctest --test-dir build -R '^tst_(selection_api|selection_alt|selection_move|selection_preview|color_range|refine_edge|match_color|replace_color|channels_panel|paint_api|fill_tools|eraser_tools|retouch_tools|edit_clipboard|export_as|save_for_web|save_format_options|image_ops|open_save)$' --output-on-failure
```

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `cp_after_edit` | Commit a pixel-changing edit through the edit transaction, then request the current picture | The answer is the document's composite, not a newly composited stack | `pictura-app::picture_is_served_after_a_committed_edit` |
| `cp_after_preview` | Present a direct opacity or fill preview and a transaction preview, then request the current picture | The answer is the document's composite, which includes the preview | `pictura-app::picture_is_served_after_a_preview` |
| `cp_after_history` | Undo, redo, and jump through history, requesting the current picture each time | The answer is the restored document's composite | `pictura-app::picture_is_served_after_history_navigation` |
| `cp_layer_stroke` | Start a paint stroke, sample a dab, skip the present, request the current picture | A fresh composite that contains the dab, not the maintained composite that lags it | `pictura-app::a_live_layer_stroke_is_composited` |
| `cp_mask_stroke` | Start a mask stroke, sample a dab, skip the present, request the current picture | The accessor composites rather than serving the cached frame (the mask is not written back until commit) | `pictura-app::a_live_mask_stroke_is_composited` |
| `cp_lifted_transform` | Begin a lifted free transform, request the current picture | A fresh composite of the lifted stack, not the composite from before the lift | `pictura-app::a_lifted_transform_is_composited` |
| `cp_non_rgb` | Convert a layered document to Grayscale and request the current picture | A four-channel composite, not the document's one-plane composite | `pictura-app::a_grayscale_document_is_composited` |
| `cp_size_mismatch` | Set a layered document's composite to different dimensions, request the current picture | A fresh composite at the document's dimensions | `pictura-app::a_mismatched_composite_is_composited` |
| `cp_layerless` | Request the current picture of a layerless document | The embedded composite, not an empty layer-stack composite | `pictura-app::a_layerless_document_keeps_its_embedded_composite` |
| `cp_identical` | Request the current picture for a fresh, a previewed, a live-stroke, a lifted-transform, a non-RGB, a layerless, and a mismatched-composite document | Equal to a full composite for the fresh and previewed states, to `current_buffer` for the fallback states, and the embedded composite when layerless | `pictura-app::the_served_picture_equals_a_full_composite` |
| `cp_consumer` | Sample a fresh document through the Sample All Layers surface; then click Magic Wand, drag Quick Selection, Copy Merged, and Save on a large layered document | The sampling surface is the document's composite with a zero composite delta; the GUI consumers respond without the full-composite stall and produce the same result as before | `pictura-app::a_read_only_consumer_composites_nothing`; manual: the Magic Wand, Quick Selection, Copy Merged, and Save entry points are GUI-only and their stall cannot be asserted in CI |
