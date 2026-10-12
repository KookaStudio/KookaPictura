# Tasks

## 1. Docs first and spike (gates unit D)

- [x] 1.1 On a separate docs commit with `TASK-ALLOWS-DOCS`, land issue #281: add
  `docs/dev/refactor-view-seam.md`, mark Track 4 in
  `docs/dev/refactor-tool-architecture.md` superseded, add adapter terms to
  `docs/GLOSSARY.md` — verify `bash scripts/guard.sh` passes
- [x] 1.2 Spike `Vec<LayerRow>`: one cxx shared struct `LayerRow`, try
  `#[qinvokable] layers_snapshot -> Vec<LayerRow>` and free-fn fallback; record the
  chosen vehicle in `design.md` D1 — verify `cmake --build build` succeeds on the
  chosen path
- [x] 1.3 Create `crates/pictura-app/cpp/api/` layout and list a placeholder pair (or
  the first real unit pair) in `CMakeLists.txt` — verify configure + build succeeds

## 2. Unit A — document + history API

- [x] 2.1 Implement `document_api` and `history_api` as unit façades (centralize
  open/new/save/dirty, display-level size as typed `CanvasSize`, undo/redo/history
  navigation and snapshots as one include surface) — verify a Qt Test covers open +
  undo/redo through the adapter and typed level size equals parsed
  `display_level_size`
- [x] 2.2 Enumerate document/history consumers with `rg -l 'PictureView|display_level_size|history_|can_undo|can_redo|document_' crates/pictura-app/cpp -g '*.cpp'` and migrate them (frame menus, session, control server document/history verbs, dialogs, navigator/channels/histogram panels for level size) — verify no production TU outside `cpp/api/` calls `display_level_size` or includes document/history `*.cxxqt.h` for those symbols; build passes
- [x] 2.3 Run both self-test invocations (`./build/pictura --headless --self-test` and
  `./build/pictura --headless --self-test crates/pictura-codec/tests/fixtures/two_layers.psd`)
  and the unit's tests — verify exit codes unchanged vs `master` baseline recorded
  before the branch

## 3. Unit B — selection API

- [x] 3.1 Add typed `SelectionContour` and `SelectionBounds` and `selection_api`
  façade; convert once at the adapter — verify a Qt Test asserts typed values equal
  the legacy `selection_contour()` / `selection_bounds()` string on a fixture
  selection
- [x] 3.2 Enumerate selection consumers (`rg -l 'selection_contour|selection_bounds|select_all|deselect|begin_lasso|select_rect|select_ellipse|magic_wand|quick_select' crates/pictura-app/cpp`) and migrate them (frame select menus, selection tools, marching-ants path in `frame.cpp`/`image_view.cpp`, `control_server_actions.cpp` selection verbs, `frame_menus_image.cpp`, `info_panel.cpp`) — verify `rg` shows no raw `selection_contour()`/`selection_bounds()` parse outside `cpp/api/` and build passes
- [x] 3.3 Once last caller is gone, delete `selection_contour` and `selection_bounds`
  string invokables and migrate their self-test string assertions
  (`selftest_tools_selection.cpp`, `selftest_tool_canvas.cpp`) — verify
  `cmake --build build`, both self-test runs with unchanged codes, and no new
  `ST_BEGIN` (budget check)

## 4. Unit C — paint + transform API (Stroke, typed geometry)

- [x] 4.1 Implement `paint_api::Stroke` as a tool-lifetime object (construct on press,
  `finish()`/`cancel()` on release/Esc, destructor closes once); mid-stroke present
  queues `flush_present` and never calls `image()`; migrate `begin_brush` helper and
  `frame.cpp` deferred `flush_present` timer into `paint_api` — verify a Qt Test
  covers early-return and release paths closing the stroke exactly once with history
  equal to explicit begin/end (no new self-test `ST_BEGIN`)
- [x] 4.2 Add typed `LayerRect`, `Quad`, and preview matrix to `transform_api`
  (and `CanvasSize` already in A); claim or defer `composite_bounds` with a named
  decision — verify equivalence tests: typed values match parsed `layer_rect` /
  `transform_quad` / `transform_preview_matrix` on fixtures, including
  `properties_panel.cpp` and `image_view.cpp` parsers
- [x] 4.3 Enumerate paint/transform consumers (`rg -l 'begin_paint|end_paint|flush_present|begin_brush|transform_quad|transform_preview_matrix|layer_rect|composite_bounds' crates/pictura-app/cpp -g '*.cpp'`) and migrate them (all `tool_*.cpp` paint tools, `tool_transform.cpp`, `image_view.cpp` matrix/quad parsers, `tool_crop.cpp`/`tool_perspectivecrop.cpp` if composite_bounds claimed, control server paint verbs) — verify `rg` gate: production TUs outside `cpp/api/` do not call raw paint bridge entry points or re-parse transform strings for migrated paths; both self-test runs unchanged codes
  - transform: all `layer_rect`/`transform_quad`/`transform_preview_matrix` consumers
    migrated; `image_view.cpp` quad/matrix parsers now typed; no raw transform-string
    re-parse outside `cpp/api/`; `composite_bounds` deferred (task 4.2).
  - paint: `tool_brush`, `tool_eraser`, `tool_retouch`, `tool_stamps`,
    `tool_colorreplacement`, `tool_mixerbrush` now drive `paint_api::Stroke`, and
    `frame.cpp`'s deferred present routes through `paint_api::presentDeferred`; no
    production TU outside `cpp/api/` calls `begin_paint`/`end_paint`/`flush_present`/
    `begin_brush`/`paint_dab` (the control server has no paint verbs). Test TUs
    (`cpp/tests/**`, `selftest_*`) keep their direct calls, which still exist.
- [x] 4.4 Delete superseded string invokables for unit C when last callers are gone
  and migrate matching Qt Test string assertions (`tst_transform_options`,
  `tst_edit_shortcuts`, `tst_crop_snap` if applicable) — verify build + both
  self-test runs + `cargo nextest run --workspace` green
  - `layer_rect`/`transform_quad`/`transform_preview_matrix` deleted; consumers now
    use `transform_api` (typed crossings live in the transform bridge, so
    `cxxqt_object.rs` shrank to 1214 at unit C; unit D took it to 1180 and 7.2 removed
    the allowlist entry entirely); `composite_bounds` deferred (task 4.2), so
    `tst_crop_snap` is not applicable

## 5. Unit D — layers API + snapshot

- [x] 5.1 Implement `layers_snapshot(thumb_size, entire_doc)` on the vehicle chosen in
  task 1.2, returning typed rows including style-effect names, thumbnails, and
  vector-mask fields — verify build with the spike vehicle (qinvokable or free-fn)
  still green
  - vehicle validated: a widened `LayerRow` (QString/i32/bool/`QImage` fields) compiles
    in `#[qinvokable] -> Vec<LayerRow>`. cxx rejects `Vec<QString>` as a shared-struct
    field (`QString: ImplVec` unsatisfied), so `style_effects` crosses as one `QString`
    joined by `\n`, split back in the adapter. Filling every field + deleting the 29
    `layer_row_*` entries (5.5) must land together to keep `cxxqt_object.rs` under its
    1214 cap.
- [x] 5.2 Add `tst_layers_snapshot` (register explicitly in
  `crates/pictura-app/cpp/tests/CMakeLists.txt` — no globbing) asserting snapshot ≡
  legacy per-getter output for every field on a fixture document covering styled,
  vector-masked, smart-object, clipped, shape, nested, and adjustment rows — verify
  `ctest --test-dir build -R '^tst_layers_snapshot$' --output-on-failure`
  - the field-for-field equivalence against the legacy getters was asserted while
    both surfaces existed; 5.5 removed the getters, so the surviving
    `fixtureExpectations` / `entireDocumentPlacement` cases carry the typed-value
    checks (the equivalence half retired with the getters, as the later unit planned).
- [x] 5.3 Implement `layers_api` (row model façade, layer ops already on the bridge)
  and migrate `layers_panel` rebuild to one snapshot per refresh — verify panel
  refresh path has no per-field `layer_row_*` loop; existing layers Qt Tests pass
- [x] 5.4 Enumerate remaining `layer_row_*` / style-probe C++ callers (`rg -l
  'layer_row_|layer_style_value|shape_row_' crates/pictura-app/cpp`) and migrate them
  (properties panel, menus, control server layer verbs, self-tests asserting row
  getters) — verify `rg 'layer_row_'` outside `cpp/api/` is empty for C++ consumers
  (Rust entry points still exist until 5.5)
- [x] 5.5 Delete the **29** `layer_row_*` entry points (20 `PictureView` invokables +
  9 free functions) and the N-effect `layer_style_value` row loop from consumers —
  verify `rg 'fn layer_row_' crates/pictura-app/src` is empty for deleted surface,
  build, both self-test runs unchanged codes, `cargo nextest run --workspace`, and
  layers Qt Tests green

## 6. Unit E — type API

- [x] 6.1 Implement `type_api` as a unit façade over type/text surface: the 31
  `type_tools` free functions (preview, commit, edit session, style CRUD) plus the
  `rasterize_type`/`layer_is_type` bridge verbs used by panels — name the multi-call
  sequences centralized (edit_begin/commit/cancel, style apply) so it is not a 1:1
  pass-through — verify a Qt Test covers type commit + style apply through the adapter
- [x] 6.2 Enumerate type consumers (`rg -l 'type_|rasterize_type|layer_is_type|
  CharacterSetting|ParagraphSetting' crates/pictura-app/cpp -g '*.cpp'`) and migrate
  them (character panel, paragraph panel, type tool, `tst_type_tools`) — verify `rg`
  gate for raw type invokables outside `cpp/api/` and both self-test runs unchanged
  codes

## 7. Integration and verification

- [x] 7.1 Full bar: `cmake --build build`; both `./build/pictura --headless
  --self-test` runs (bare and `crates/pictura-codec/tests/fixtures/two_layers.psd`);
  `cargo nextest run --workspace`; `cargo test --workspace --doc`;
  `ctest --test-dir build -R '^tst_' --output-on-failure`; `cargo fmt --all --check`;
  `cargo clippy --workspace --all-targets -- -D warnings` — verify all pass with
  self-test exit codes unchanged vs baseline
- [x] 7.2 `openspec validate --all --strict`; `bash scripts/check-file-size.sh`;
  `bash scripts/guard.sh`; `bash scripts/check-selftest-budget.sh` — verify all pass;
  if `cxxqt_object.rs` is under 1200 LOC, remove or lower its
  `scripts/file-size-allowlist.txt` entry; selftest budget must not rise
  - `cxxqt_object.rs` is 1180 (< 1200); its allowlist entry was removed.
    `selftest.cpp` grew by the one added adapter include and reclaimed that line,
    so its 6449 ceiling holds. Selftest budget 510/511 (no new `ST_BEGIN`).
- [x] 7.3 Confirm exclusive-surface claim and record deferrals: `rg -l
  'cxxqt_object.*\.cxxqt\.h'` under `crates/pictura-app/cpp` only lists `cpp/api/**`
  plus named deferrals (uncovered bridge operations and any `frame_includes.h` lines
  whose unit has not landed, e.g. export if unowned) — list every leftover in the PR
  description matching the requirement's named-deferral clause. Deferral recorded by
  unit B: the selection geometry (`selection_contour`/`selection_bounds`) is the
  selection surface unit B covers; the selection verbs (`select_all`, `deselect`,
  `select_rect`, `select_ellipse`, the lasso trio, `magic_wand`, `quick_select`, the
  modify/grow/similar/save/load ops, `  select_layer_alpha`, and the selection-move
  sequence) remain direct bridge calls until a unit claims them. Deferral recorded by
  unit C: `composite_bounds` stays a string crossing for `tool_crop` /
  `tool_perspectivecrop` (a crop-specific follow-up).
  - Deferral recorded by units D/E (integration): the per-unit generated headers are
    confined to `cpp/api/**` — `type_tools.cxxqt.h` appears only in `api/type_api.h`;
    no raw `layer_row_*`/`type_*` bridge entry point is called outside `cpp/api/**`.
    Every remaining generated-header include outside `cpp/api/**` is an uncovered
    bridge operation (the god `cxxqt_object.cxxqt.h` plus per-operation headers:
    `annotations`, `paths`/`path_list`, `guides`, `crop_group`, `paint_tools`(+`fills`),
    `healing`, `layer_masks`, `vector_masks`, `clipping`, `shapes`, `layer_style`,
    `layers_surface`, `layers_smart_filters`, `smart_object_actions`, `image_adjust/*`
    (`new_document`, `image_size`, `image_mode`, `image_ops`, `gradient_map`),
    `image_replace_color`, `image_match_color`, `image_hdr_toning`,
    `filter_tools`(+`jobs`), `export`, `web_export`, `channels`, `color_range`,
    `impl_selection/refine`, `magnetic`, `eyedropper_sample`, `align`,
    `layer_arrange`, `matting`, `clipboard`, `impl_history` (`purge`, `display`),
    `impl_core/recovery`, `impl_transform/bridge`). `frame_includes.h` keeps
    `new_document`/`clipping`/`layer_masks` because the frame TUs use those headers for
    operations no landed unit covers (`create_document`, `clipping_*`, `layer_mask_*`);
    removing them breaks `frame.cpp`/`frame_menus.cpp`, so they are named deferrals.
    `export` remains unowned. All are listed for the PR description.
  - Named exception to the "generated headers only in adapter TUs" clause:
    `cpp/api/type_api.h` includes `type_tools.cxxqt.h` because the type structs
    (`CharacterSetting`, `ParagraphSetting`, `TypeSetting`) are part of the adapter's
    parameter interface, so consumers of `api/type_api.h` transitively include the
    generated header. Design D2 permits an adapter header to depend on cxx-qt types
    for parameters; recorded here as the one sanctioned exception.