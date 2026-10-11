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

- [ ] 2.1 Implement `document_api` and `history_api` as unit façades (centralize
  open/new/save/dirty, display-level size as typed `CanvasSize`, undo/redo/history
  navigation and snapshots as one include surface) — verify a Qt Test covers open +
  undo/redo through the adapter and typed level size equals parsed
  `display_level_size`
- [ ] 2.2 Enumerate document/history consumers with `rg -l 'PictureView|display_level_size|history_|can_undo|can_redo|document_' crates/pictura-app/cpp -g '*.cpp'` and migrate them (frame menus, session, control server document/history verbs, dialogs, navigator/channels/histogram panels for level size) — verify no production TU outside `cpp/api/` calls `display_level_size` or includes document/history `*.cxxqt.h` for those symbols; build passes
- [ ] 2.3 Run both self-test invocations (`./build/pictura --headless --self-test` and
  `./build/pictura --headless --self-test crates/pictura-codec/tests/fixtures/two_layers.psd`)
  and the unit's tests — verify exit codes unchanged vs `master` baseline recorded
  before the branch

## 3. Unit B — selection API

- [ ] 3.1 Add typed `SelectionContour` and `SelectionBounds` and `selection_api`
  façade; convert once at the adapter — verify a Qt Test asserts typed values equal
  the legacy `selection_contour()` / `selection_bounds()` string on a fixture
  selection
- [ ] 3.2 Enumerate selection consumers (`rg -l 'selection_contour|selection_bounds|select_all|deselect|begin_lasso|select_rect|select_ellipse|magic_wand|quick_select' crates/pictura-app/cpp`) and migrate them (frame select menus, selection tools, marching-ants path in `frame.cpp`/`image_view.cpp`, `control_server_actions.cpp` selection verbs, `frame_menus_image.cpp`, `info_panel.cpp`) — verify `rg` shows no raw `selection_contour()`/`selection_bounds()` parse outside `cpp/api/` and build passes
- [ ] 3.3 Once last caller is gone, delete `selection_contour` and `selection_bounds`
  string invokables and migrate their self-test string assertions
  (`selftest_tools_selection.cpp`, `selftest_tool_canvas.cpp`) — verify
  `cmake --build build`, both self-test runs with unchanged codes, and no new
  `ST_BEGIN` (budget check)

## 4. Unit C — paint + transform API (Stroke, typed geometry)

- [ ] 4.1 Implement `paint_api::Stroke` as a tool-lifetime object (construct on press,
  `finish()`/`cancel()` on release/Esc, destructor closes once); mid-stroke present
  queues `flush_present` and never calls `image()`; migrate `begin_brush` helper and
  `frame.cpp` deferred `flush_present` timer into `paint_api` — verify a Qt Test
  covers early-return and release paths closing the stroke exactly once with history
  equal to explicit begin/end (no new self-test `ST_BEGIN`)
- [ ] 4.2 Add typed `LayerRect`, `Quad`, and preview matrix to `transform_api`
  (and `CanvasSize` already in A); claim or defer `composite_bounds` with a named
  decision — verify equivalence tests: typed values match parsed `layer_rect` /
  `transform_quad` / `transform_preview_matrix` on fixtures, including
  `properties_panel.cpp` and `image_view.cpp` parsers
- [ ] 4.3 Enumerate paint/transform consumers (`rg -l 'begin_paint|end_paint|flush_present|begin_brush|transform_quad|transform_preview_matrix|layer_rect|composite_bounds' crates/pictura-app/cpp -g '*.cpp'`) and migrate them (all `tool_*.cpp` paint tools, `tool_transform.cpp`, `image_view.cpp` matrix/quad parsers, `tool_crop.cpp`/`tool_perspectivecrop.cpp` if composite_bounds claimed, control server paint verbs) — verify `rg` gate: production TUs outside `cpp/api/` do not call raw paint bridge entry points or re-parse transform strings for migrated paths; both self-test runs unchanged codes
- [ ] 4.4 Delete superseded string invokables for unit C when last callers are gone
  and migrate matching Qt Test string assertions (`tst_transform_options`,
  `tst_edit_shortcuts`, `tst_crop_snap` if applicable) — verify build + both
  self-test runs + `cargo nextest run --workspace` green

## 5. Unit D — layers API + snapshot

- [ ] 5.1 Implement `layers_snapshot(thumb_size, entire_doc)` on the vehicle chosen in
  task 1.2, returning typed rows including style-effect names, thumbnails, and
  vector-mask fields — verify build with the spike vehicle (qinvokable or free-fn)
  still green
- [ ] 5.2 Add `tst_layers_snapshot` (register explicitly in
  `crates/pictura-app/cpp/tests/CMakeLists.txt` — no globbing) asserting snapshot ≡
  legacy per-getter output for every field on a fixture document covering styled,
  vector-masked, smart-object, clipped, shape, nested, and adjustment rows — verify
  `ctest --test-dir build -R '^tst_layers_snapshot$' --output-on-failure`
- [ ] 5.3 Implement `layers_api` (row model façade, layer ops already on the bridge)
  and migrate `layers_panel` rebuild to one snapshot per refresh — verify panel
  refresh path has no per-field `layer_row_*` loop; existing layers Qt Tests pass
- [ ] 5.4 Enumerate remaining `layer_row_*` / style-probe C++ callers (`rg -l
  'layer_row_|layer_style_value|shape_row_' crates/pictura-app/cpp`) and migrate them
  (properties panel, menus, control server layer verbs, self-tests asserting row
  getters) — verify `rg 'layer_row_'` outside `cpp/api/` is empty for C++ consumers
  (Rust entry points still exist until 5.5)
- [ ] 5.5 Delete the **29** `layer_row_*` entry points (20 `PictureView` invokables +
  9 free functions) and the N-effect `layer_style_value` row loop from consumers —
  verify `rg 'fn layer_row_' crates/pictura-app/src` is empty for deleted surface,
  build, both self-test runs unchanged codes, `cargo nextest run --workspace`, and
  layers Qt Tests green

## 6. Unit E — type API

- [ ] 6.1 Implement `type_api` as a unit façade over type/text surface: the 31
  `type_tools` free functions (preview, commit, edit session, style CRUD) plus the
  `rasterize_type`/`layer_is_type` bridge verbs used by panels — name the multi-call
  sequences centralized (edit_begin/commit/cancel, style apply) so it is not a 1:1
  pass-through — verify a Qt Test covers type commit + style apply through the adapter
- [ ] 6.2 Enumerate type consumers (`rg -l 'type_|rasterize_type|layer_is_type|
  CharacterSetting|ParagraphSetting' crates/pictura-app/cpp -g '*.cpp'`) and migrate
  them (character panel, paragraph panel, type tool, `tst_type_tools`) — verify `rg`
  gate for raw type invokables outside `cpp/api/` and both self-test runs unchanged
  codes

## 7. Integration and verification

- [ ] 7.1 Full bar: `cmake --build build`; both `./build/pictura --headless
  --self-test` runs (bare and `crates/pictura-codec/tests/fixtures/two_layers.psd`);
  `cargo nextest run --workspace`; `cargo test --workspace --doc`;
  `ctest --test-dir build -R '^tst_' --output-on-failure`; `cargo fmt --all --check`;
  `cargo clippy --workspace --all-targets -- -D warnings` — verify all pass with
  self-test exit codes unchanged vs baseline
- [ ] 7.2 `openspec validate --all --strict`; `bash scripts/check-file-size.sh`;
  `bash scripts/guard.sh`; `bash scripts/check-selftest-budget.sh` — verify all pass;
  if `cxxqt_object.rs` is under 1200 LOC, remove or lower its
  `scripts/file-size-allowlist.txt` entry; selftest budget must not rise
- [ ] 7.3 Confirm exclusive-surface claim and record deferrals: `rg -l
  'cxxqt_object.*\.cxxqt\.h'` under `crates/pictura-app/cpp` only lists `cpp/api/**`
  plus named deferrals (uncovered bridge operations and any `frame_includes.h` lines
  whose unit has not landed, e.g. export if unowned) — list every leftover in the PR
  description matching the requirement's named-deferral clause