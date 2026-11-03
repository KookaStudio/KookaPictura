# Tasks

## 1. Tests first

- [x] 1.1 Add `a_dab_outside_the_clip_moves_nothing`,
  `a_clip_leaves_the_picture_outside_it_untouched`,
  `a_clipped_view_is_clear_outside_its_bounds`, and extend
  `a_mesh_scaled_to_a_larger_layer_scales_its_clip` in
  `crates/pictura-filters/src/distort/liquify/tests.rs`; confirm they fail
  before the clip exists.
- [x] 1.2 Add `session_clip_refuses_a_selection_off_the_layer` in
  `crates/pictura-app/src/cxxqt_object/filter_tools/liquify/imp.rs` against a
  pure `session_clip` helper; confirm it fails before the helper exists.
- [x] 1.3 Add `aSelectionConfinesThePreviewAndCommitToItsBounds` and
  `aNonRectangularSelectionProcessesItsBoundingBox` to
  `crates/pictura-app/cpp/tests/tst_liquify_dialog.cpp`; confirm they fail
  before the session and canvas changes.

## 2. Mesh clip

- [x] 2.1 Add `clip: Option<PsdRect>` to `LiquifyMesh` with `clip()`,
  `set_clip()`, and a private `contains()`; `set_clip` clamps to the layer and
  zeroes offsets outside. `displacement` returns zero outside, `dab` skips nodes
  outside, and the returned dirty rectangle is intersected with the clip.
- [x] 2.2 `scaled_to` scales the clip (floor/ceil outward) and `grid_lines`
  intersects its rectangle with it.
- [x] 2.3 `liquify()` resamples only the clip's rows and columns, copying the
  source row for the rest so pixels outside are bit-identical.
- [x] 2.4 `LiquifySource::render` clears pixels whose layer point is outside the
  clip, for both the full-resolution and the reduced-level paths, and tints the
  freeze overlay only inside it.

## 3. Commit confinement

- [x] 3.1 In `pictura-render::apply_filter_region`, drop the coverage mask for
  `Filter::Liquify` when its mesh has a clip, so the commit processes the whole
  rectangle rather than the selection's shape.

## 4. Session and dialog

- [x] 4.1 Add `session_clip(selection, layer_rect)` to the Liquify session
  module: no selection is the whole layer, a covering or overlapping selection
  is its layer-local intersection, an empty selection or one that misses the
  layer is refused.
- [x] 4.2 `liquify_begin` computes the clip, refuses when `session_clip` does,
  builds the mesh with it, and stores it on the session; `liquify_load_mesh`
  replaces a loaded mesh's clip with the session's.
- [x] 4.3 `liquify_geometry` reports the clip size and the new `liquify_origin`
  reports its top-left; `liquify_backdrop` crops to the same frame.
- [x] 4.4 `LiquifyCanvas` gains `origin_`, maps `toLayer`/`toWidget` through it,
  and uses it for the paint target, the mesh transform, and the mesh trace.

## 5. Verification and archive

- [x] 5.1 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo nextest run -p pictura-filters -p pictura-render
  -p pictura_app`, and `ctest --test-dir build -R '^tst_liquify_dialog$'
  --output-on-failure`.
- [x] 5.2 Run `python3 scripts/check-test-plan.py` and `openspec validate --all
  --strict`.
- [x] 5.3 Update `docs/dev/STATE.md` and commit the docs with
  `TASK-ALLOWS-DOCS`.
- [x] 5.4 Archive the change with `openspec archive liquify-selection-bounds`
  and confirm `scripts/check-openspec-archived.sh` passes.
