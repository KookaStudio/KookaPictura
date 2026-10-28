# Tasks

## 1. Tests first

- [x] Add `tst_layers_panel_fixes::adjustmentThumbnailIsSquareWithoutOutline` —
  a portrait document, an Invert row: the card is 1:1, the icon is not stretched,
  and there is no black outline; a pixel row on the same document stays
  letterboxed and outlined.
- [x] Add `tst_layers_panel_fixes::fillThumbnailIsSquareWithOutline` — a portrait
  document, a Solid Color fill row: the card is 1:1, the preview fills it (no
  document-aspect margin), and the black outline is kept.
- [x] Add `tst_layers_panel_fixes::maskThumbnailMatchesSquareCard` — an adjustment
  and a fill row each carrying a mask: both mask cards use the same 1:1 rect as
  the image card.
- [x] Add `tst_layers_snapshot::fillThumbnailIsSquare` — a non-square document's
  fill row preview from `layers_api::snapshot` is square.
- [x] Add `tst_layers_effects::effectsRowHasNoOutline` — a styled layer's
  `Effects` group row is square and carries no black outline.
- [x] Add `tst_layers_smart_filters::smartFiltersRowHasNoOutline` — the
  `Smart Filters` group row is square and carries no black outline.

## 2. Delegate

- [x] Add the `iconOnlyRow()` / `squareThumbCard()` predicates (a type card, an
  adjustment-kind icon, a content-less synthetic row, or a group is icon-only;
  a fill row squares too; a group never squares).
- [x] Use `squareThumbCard()` in `thumbnailCard()`, and switch `maskThumbRect()` /
  `vectorMaskThumbRect()` to `thumbnailCard()`.
- [x] Draw the adjustment icon and the Smart Filters / layer-effects `fx` badge
  into `shaped` (the square card), not `box`.
- [x] Draw the outline only when the row is not icon-only (a raster image card).
- [x] Draw the mask and vector-mask previews fit inside their card.

## 3. Bridge

- [x] Render `fill_thumbnail_image` at `(size, size)` and drop its unused
  `entire_document` argument; update the `impl_layers.rs` call.

## 4. Docs and verification

- [x] Add a `docs/dev/STATE.md` entry (the commit carries `TASK-ALLOWS-DOCS`).
- [x] Run `openspec validate --all --strict`, the focused Qt suites, and archive
  the change.
