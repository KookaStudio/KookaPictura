# Tasks: isolation-mode

## 1. Contract

- [x] 1.1 `docs/05-layers/layer-filtering-and-search.md`: move `Selected` from the
  CC-only exclusion into scope; add the isolation-mode behavior and provenance.
- [x] 1.2 `docs/02-ui-ux/panels/layers-panel.md` and `docs/GLOSSARY.md`: the
  Selected filter type and the isolation-mode term.

## 2. The Selected filter

- [x] 2.1 `layers_filter_bar.{h,cpp}`: add the `selected` dimension with an empty
  criteria page and `buildFilter` setting `filter.selected`.
- [x] 2.2 `layers_filter_proxy.{h,cpp}`: `LayerFilter::selected` +
  `selectedPaths`; `hasActiveCriteria` and `rowMatches` handle it via `PathRole`.
- [x] 2.3 `layers_panel.{h,cpp}`: capture the selection as the fixed set on
  entry; `enterIsolation()` / `releaseIsolation()`.

## 3. Move Auto-Select scoping

- [x] 3.1 `state.rs` + `impl_core.rs` + `cxxqt_object.rs`: the
  `isolation: Option<Vec<String>>` set and the `isolation_set(paths, active)`
  bridge; `impl_core.rs` reset.
- [x] 3.2 `helpers_composite.rs::auto_select_path` takes the set and filters
  members/descendants; `impl_layers/auto_select.rs` passes it.
- [x] 3.3 Rust `#[test]`s for the scoped hit test (`tests_auto_select.rs`).
- [x] 3.4 Panel pushes the set on entry/release.

## 4. Release and entry/exit

- [x] 4.1 `layers_panel_menu.cpp`: a `Release from Isolation` row shown while
  isolated, dropping the row (last release ends isolation).
- [x] 4.2 `frame.cpp` / `frame_menus_select.cpp`: the Move-tool canvas right-click
  `Isolation Mode` entry; Escape releases isolation.

## 5. Verification

- [x] 5.1 Qt Test `tst_layers_panel::isolationMode`; `ctest -R '^tst_'` 89/89 and
  the app self-test 474.
- [x] 5.2 `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` (only the pre-existing
  local ImageMagick fixture mismatch fails) and `openspec validate --all
  --strict` 260/260.
