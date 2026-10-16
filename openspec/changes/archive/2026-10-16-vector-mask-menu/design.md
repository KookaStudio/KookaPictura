# Design

- **Menu:** `showContextMenu` checks the vector-mask thumbnail rectangle after
  the layer-mask one and fills the menu with `populateVectorMaskMenu`. Its
  actions run through `performMaskAction` next to the layer-mask ones, via new
  path bridges `vector_mask_delete_path` and `vector_mask_rasterize_path`. The
  toggle label and action read the document through `vector_mask_disabled_path`
  rather than the panel model, which lags until a refresh.
- **Trash:** `trashVectorMaskPath()` returns the sole selected path when that
  layer has both masks (`layer_mask_present_path`, `vector_mask_present_path`).
  Both the trash's enabled state and its action use it. The active layer
  follows the current row, which Ctrl+arrow can move outside the selection, so
  reading the active layer could delete another layer's vector mask, or delete
  the selected layer without asking.
- **Prompt:** a `QMessageBox` with a "Don't show again" check box; the opt-out
  is `SessionState::confirmDeleteVectorMask`, re-loaded before saving so a
  concurrent session write is not clobbered.
