# Tasks

## 1. Tests named by the plan

- [x] 1.1 Qt: add `tst_present_cache::soloHideRefreshesTheWholeCanvas` (solo hide and restore at fit zoom).
- [x] 1.2 Qt: add `tst_present_cache::openedBackgroundHidesAtFitZoom` (opened Background at a reduced level).
- [x] 1.3 Qt: add `tst_present_cache::lazyOpenedBackgroundHidesAtFitZoom` (merged preview dropped).
- [x] 1.4 `pictura-app`: add `visibility_change_region_covers_the_layers_solo_hides`.

## 2. Visibility refresh

- [x] 2.1 Compute the changed nodes' reach before `apply_visibility` mutates the tree.
- [x] 2.2 Use that region for the edit, falling back to `Edit::Full` when any changed node is unbounded.

## 3. Close-out

- [x] 3.1 `openspec validate --all --strict`, `check-test-plan.py`, one build, `verify-fast` (one unrelated ImageMagick oracle fixture failure on this host).
- [x] 3.2 Archive the change.
