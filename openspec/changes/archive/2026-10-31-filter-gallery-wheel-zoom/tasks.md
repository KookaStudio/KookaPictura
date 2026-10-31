# Tasks: filter-gallery-wheel-zoom

## 1. Test plan and tests first

- [x] 1.1 Write `test-plan.md` mapping every scenario in
  `specs/imaging/filter-gallery/spec.md` to a coverage cell.
- [x] 1.2 Add `tst_filter_gallery::wheelZoomAnchorsAtTheCursor` (fails before
  the fix): pan at 200 %, wheel up at a point away from the centre, assert the
  document point under it is unchanged within one pixel; then wheel down and
  assert the centre's document point is unchanged.
- [x] 1.3 Add `tst_filter_gallery::buttonsAndDropdownAnchorAtTheCentre` (fails
  before the fix): pan, then zoom with the `−`/`+` buttons and the drop-down and
  assert the centre's document point is unchanged within one pixel.

## 2. Anchored zoom

- [x] 2.1 Add `documentPointAt(viewportPoint)` (canvas position plus zoom) and
  `setZoomAt(zoom, anchor)` to `FilterGalleryDialog`; after `layoutPreview`, move
  each scrollbar by the measured drift so the point lands at the anchor.
- [x] 2.2 Route `setZoom` (drop-down, typed percentage) and `stepZoom` (buttons,
  shortcuts) through the viewport centre; pass `QWheelEvent::position()` on
  zoom-in and the centre on zoom-out.

## 3. Sibling preview audit

- [x] 3.1 Check `LiquifyCanvas`, `AwaCanvas`, `VpCanvas`, and
  `LensCorrectionCanvas`: each already calls `zoomAbout(zoom, event->position())`
  on the wheel and `zoomAbout(zoom, widget centre)` from `setZoom`. No change.

## 4. Verification

- [x] 4.1 Build once and run `tst_filter_gallery` (18 pass; both new cases fail
  on the unfixed dialog), the wider Qt layer (2,128 pass), and the Rust
  workspace. The report's only failures are the pre-existing local
  `ramp_Divide.rgba` ImageMagick fixture mismatch and the self-test's exit 283,
  both documented in `docs/dev/STATE.md` and unrelated to this change.
- [x] 4.2 `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --
  -D warnings`, `scripts/guard.sh`, `openspec validate --all --strict`,
  `scripts/check-test-plan.py`, and `scripts/check-file-size.sh` pass.

## 5. Close out

- [x] 5.1 Archive the change (`openspec archive filter-gallery-wheel-zoom`).
- [x] 5.2 Add the #840 entry to `docs/dev/STATE.md` and commit it with
  `TASK-ALLOWS-DOCS`.
