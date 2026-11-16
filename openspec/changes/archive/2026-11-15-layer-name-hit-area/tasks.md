# Tasks

## 1. Tests

- [x] 1.1 Add `tst_layers_panel::renameHitAreaMatchesRenderedLabel` for a short
  name: the label rect is narrower than the row, a double-click inside it opens
  the editor, and a double-click to its right opens Layer Style with no editor.
- [x] 1.2 Add a test seam that double-clicks just right of the rendered label.

## 2. App

- [x] 2.1 Replace the full-band `nameRect` width with the elided text width,
  floored to one pixel.
- [x] 2.2 Draw the regular and synthetic row names through the same rect.
- [x] 2.3 Update the rename-band self-test wording to the rendered label.

## 3. Gates

- [x] 3.1 Build and run `tst_layers_panel` and the application self-test.
- [x] 3.2 `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`.
- [x] 3.3 Archive with `openspec archive layer-name-hit-area`.
