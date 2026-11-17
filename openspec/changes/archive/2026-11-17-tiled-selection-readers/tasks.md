# Tasks

## 1. Tests first

- [x] 1.1 Parity + no-materialization tests per test-plan row (render, paint, select, app).

## 2. Render: clipboard and layer ops

- [x] 2.1 `coverage_bounds` / `plane_has_coverage` take `&Plane<u8>`; clipboard family takes `Option<&Plane<u8>>` and reads row spans.
- [x] 2.2 Lift, Layer via Copy/Cut and `move_mask_selection_content` read row spans.

## 3. Filters and adjustments through a selection

- [x] 3.1 `filter::coverage` samples; `write_back` and the Auto coverage read mask rows.

## 4. Healing

- [x] 4.1 Patch and Content-Aware Move take `&Plane<u8>`; bounds and coverage by rows.

## 5. Selection crate leftovers

- [x] 5.1 `fit_centered` by rows; Refine Edge recorded out of scope.

## 6. App readers and writers

- [x] 6.1 Bridge exports fill one `Vec` with `read_into`.
- [x] 6.2 Call sites pass `&sel.data` as a plane; indexed reads use `sample`/rows.
- [x] 6.3 `DerefMut` writers use `write_row_from` / `write_rect_from`.
- [x] 6.4 Heap guard `selection_readers_do_not_materialize`.

## 7. Close-out

- [x] 7.1 Remove the `ponytail:` markers this resolves (`Selection` doc comment).
- [x] 7.2 `docs/dev/STATE.md` entry; archive the change.
