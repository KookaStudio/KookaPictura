# Tasks

## 1. Tests

- [x] 1.1 Add `tst_new_document::backgroundSwatchFollowsBackgroundContents`:
  the list is White / Background Color / Transparent, the swatch shows white
  and then the toolbox background, and it is disabled for Transparent.
- [x] 1.2 Add `tst_new_document::backgroundSwatchPicksAndCancelsACustomColor`:
  an accepted pick adds and selects Custom, fills the document, and a rejected
  pick leaves Custom and the fill unchanged.

## 2. App

- [x] 2.1 Replace the Background Contents entries with White, Background Color,
  and Transparent, and add the swatch beside the list.
- [x] 2.2 Drive the swatch color and enabled state from the current choice; open
  the picker and add/select Custom only on an accepted custom color.
- [x] 2.3 Refresh the German and Finnish translation catalogs.

## 3. Gates

- [x] 3.1 Build and run `tst_new_document`.
- [x] 3.2 `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`.
- [x] 3.3 Archive with `openspec archive new-document-background-color`.
