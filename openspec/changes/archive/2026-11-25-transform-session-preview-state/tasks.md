# Tasks

Line ranges refer to the pre-change tree; earlier tasks shift them, so re-read
before slicing. `impl_transform/session.rs` is already at the 1200-line cap, so
group 1 temporarily exceeds it; `check-file-size.sh` is first run in 5.4, after
the group-2 move.

## 1. One preview teardown

- [x] 1.1 Add the Rust test `a_replaced_document_clears_the_preview_fields` covering `ds_preview_replacement`: set `move_base_origin`, `move_prepared_layer`, and a non-zero `transform_preview_level`, call `replace_document`, and assert all three are cleared; verify it fails before 1.6
- [x] 1.2 Add the Rust test `a_cancelled_controls_gesture_clears_the_above_layer` covering `ds_preview_cancel_clears`: set the four above-layer fields, `mask_move_baseline`, and `move_drag_active` on a `PictureViewRust` with a live controls transform, cancel through `cancel_active_sessions`, and assert they are cleared; verify it fails before 1.5 and 1.7
- [x] 1.3 Add the Qt test `tst_transform_controls::everyTransformEndingClearsTheSamePreviewFields` covering `ds_preview_one_teardown`: drive the six endings over `PictureView` — explicit cancel, changed commit, a commit refused for memory, a failed begin, a landed deferred commit, and a smart-object commit — asserting the same four observed fields are clear each time: `move_preview_base`, `move_preview_above`, and `move_preview_layer` null and `move_preview_base_origin` empty. Create the outgoing preview and attempt the failed replacement through `ToolController`, then assert `ImageView::transformPreviewActive` is false. The refused commit and failed begin pin available memory with `pictura::set_available_memory_for_test` and un-pin it afterwards; the deferred leg opens its own 2048² document so the commit's area crosses the deferral threshold. Verify it fails before 1.5 and 1.8
- [x] 1.4 Add `PictureViewRust::clear_preview_overlay` in `cxxqt_object/state.rs` holding the twenty preview and Move assignments, including `mask_move_baseline`, and turn `clear_transform_preview` into a thin wrapper with no extra mask-move call; verify `cargo clippy -p pictura-app --all-targets -- -D warnings` is clean
- [x] 1.5 Route `cancel_transform_session` in `document_session/session.rs:821` through `clear_preview_overlay` and delete its twelve-assignment field list; verify no preview or Move field remains assigned in that function and `cargo nextest run -p pictura-app a_cancelled_controls_gesture` passes
- [x] 1.6 Route `clear_document_scoped_state` in `document_session/session.rs:734-804` through `clear_preview_overlay` and delete its duplicate `move_drag_active` and `mask_move_baseline` clears, so `move_base_origin` and the mask-move baseline are cleared with the rest; verify `cargo nextest run -p pictura-app a_replaced_document_clears` passes
- [x] 1.7 Clear `transform_controls` in both cancel paths — `cancel_transform_session` (the history restore, Undo/redo, tool switch, and document-replacement route, which never reaches `drop_transform_session`) and `drop_transform_session` — leaving `clear_preview_overlay` and the Move-tool preview paths untouched; verify `cargo nextest run -p pictura-app a_cancelled_controls_gesture` still passes
- [x] 1.8 Make the refused-begin path clear the overlay where the replaced session is dropped: `prepare_transform` (`impl_transform/session.rs:1030-1176`) takes the outgoing session and restores its `before` at `:1041-1051` but leaves that session's preview fields set, and the refusals after that point (`:1057-1064` empty lift, `:1111-1122` no rect) return with them still set; call `clear_preview_overlay` immediately after the outgoing session is taken at `:1041`, not at the `:1036` refusal, which leaves the live session in place and must keep its preview; verify the Qt test's failed-begin leg now passes
- [x] 1.9 Clear `transform_controls` when `prepare_transform` takes an outgoing session, when a new session is installed, at `commit_transform` entry, and in both cancel paths. Keep a begin refused before the outgoing take unchanged. Add `tst_transform_controls::directTransformLifecycleOwnsTheControlsFlag` for the refused-begin and direct-commit cases

## 2. The Move-preview module

- [x] 2.1 Move `impl_transform/session.rs` lines 479–971 (the Move-preview begin, duplicate, cache, getters, follow, end, and commit) plus lines 973–1024 (`compute_view_move_preview`) into a new `impl_transform/session/move_preview.rs` with no logic edits, leaving `clear_transform_preview` (`:886-908`) behind in `session.rs` as the thin wrapper; verify the file compiles and `wc -l` puts `session.rs` under 1200
- [x] 2.2 Fix the `super` paths the move shifts by one level and verify `cargo clippy -p pictura-app --all-targets -- -D warnings` is clean
- [x] 2.3 Run `bash scripts/check-file-size.sh` and verify `impl_transform/session.rs` passes with no `scripts/file-size-allowlist.txt` entry added
- [x] 2.4 Add the module to its parent's `mod` list and verify `python3 scripts/check-cmake-source-fragments.py` is unaffected and the Rust build succeeds
- [x] 2.5 Verify the Move-preview suites pass unchanged: `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R 'tst_(move|linked|large_document|damage_bounds|transform_controls)' --output-on-failure`

## 3. One transform constructor

- [x] 3.1 Add `TransformSession::new(target, path, members, orig_rect, mode, type_layer, history_label)` in `document_session/transform.rs`, computing `quad`, `start_quad`, the corners, and the three pivot fields as `prepare_transform` does today, with a unit test asserting a `Free` session's `quad` is `None` and a projective session's quad is the source corners
- [x] 3.2 Convert `prepare_transform` in `impl_transform/session.rs` to call it, passing the `type_layer` it resolves from the members and the `history_label` it derives from `pending_vector_place`; verify `cargo nextest run -p pictura-app -E 'test(session::)'` passes
- [x] 3.3 Convert `begin_selection_transform` in `impl_transform/selection/mod.rs` to call it and verify `cargo nextest run -p pictura-app transform` passes
- [x] 3.4 Convert the seven test fixtures (`distort.rs`, `skew.rs`, `perspective.rs`, `geometry.rs`, `mode_switch.rs`, `warp/imp.rs`, `session/tests.rs`) to call it and verify each module's tests pass; `skew.rs` and `distort.rs` keep their `show_pivot = false` override, and `perspective.rs`'s `free_perspective_session` still derives from its `session(mode)` helper
- [x] 3.5 Verify no literal `TransformSession {` construction remains outside `document_session/transform.rs`: `rg -n '^\s*TransformSession\s*\{' crates/pictura-app/src | rg -v 'document_session/transform.rs'` returns nothing, and that file is the only site left

## 4. One owner for the controls flag

- [x] 4.1 Add the bridge accessor `transform_controls_session(view) -> bool` in `impl_transform/bridge.rs` and its `imp` body reading the Rust flag, add the C++ wrapper in `cpp/api/transform_api.{h,cpp}` beside `previewAboveScale`, and verify both builds succeed
- [x] 4.2 Add `tst_transform_controls::controlsSessionFlagIsDerived` covering `ds_controls_flag_derived`, `tst_transform_controls::movePreviewKeepsAPromotedTransform` covering `ds_controls_flag_move_path` during both a live controls gesture and a pending transform, and `tst_transform_controls::directTransformLifecycleOwnsTheControlsFlag` covering `ds_controls_flag_direct_lifecycle`; all are behavioural guards for the derived predicate, so they pass before and after 4.3 and carry no red-test claim
- [x] 4.3 Declare `bool transformControlsSession() const;` in `cpp/tools.h` and define it in `cpp/tool_transform.cpp` as `PictureView* v = view(); return controlsIdle_ || (v && transform_controls_session(*v));`, delete the `transformControls_` member, and remove its thirteen assignments, routing their readers through the derived predicate
- [x] 4.4 Verify `rg -n transformControls_ crates/pictura-app/cpp --glob '!**/tests/**'` returns nothing — this is the "no stored copy" guard — and `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_transform_controls$' --output-on-failure` passes with the new cases

## 5. Integration

- [x] 5.1 Verify the transform suites pass unchanged: `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R 'tst_transform' --output-on-failure`
- [x] 5.2 Verify the document-session suite passes: `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_document_session$' --output-on-failure`
- [x] 5.3 Run `bash scripts/test-report.sh` and confirm the transform and Move-preview golden suites pass unchanged; `test-report.sh` reports failures and does not diff images, so any changed golden baseline is an unauthorized change and must be reverted
- [x] 5.4 Run `bash scripts/guard.sh`, `bash scripts/check-file-size.sh`, `openspec validate --all --strict`, and `python3 scripts/check-test-plan.py`
- [x] 5.5 Review `ROADMAP.md` and `PARITY.md` for coverage movement (this is a refactor, so none is expected), then archive the change: `openspec archive transform-session-preview-state`
- [x] 5.6 Run `CARGO_TARGET_DIR=build/cargo bash scripts/with-build-lock.sh -- env LC_ALL=C bash scripts/verify-fast.sh` after archiving. Fix branch-caused failures and document any failure reproduced on clean `origin/master`. Run `openspec validate --all --strict` separately because the aggregate stops at the proven baseline GPU failure.
