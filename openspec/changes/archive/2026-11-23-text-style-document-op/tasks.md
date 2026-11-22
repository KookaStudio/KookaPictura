# Tasks

## 1. Sheet-level validated edit (pictura-core)

- [x] 1.1 Add `basic_paragraph_is_never_renamed` (both rename directions refused, same-name attribute edit accepted) plus missing-source and duplicate-target rejection cases to `crates/pictura-core/src/type_tool/tests.rs`; verify the test exists with `rg -n 'fn basic_paragraph_is_never_renamed' crates/pictura-core/src/type_tool/tests.rs` (expected not to compile until task 1.2).
- [x] 1.2 Add `NamedStyleAttrs` and `TextStyleSheet::edit_style(from, to, attrs)` that validates the whole request before renaming and writing attributes, returning `StyleError::NotFound`, `Duplicate`, or `Protected` as the design specifies (protected before duplicate), and extend the `pub use type_tool::{...}` list in `crates/pictura-core/src/lib.rs` with `NamedStyleAttrs`; verify `cargo test -p pictura-core --lib type_tool` passes.

## 2. Document operation (pictura-render)

- [x] 2.1 Add the document-operation tests named in `test-plan.md` (`renaming_an_applied_paragraph_style_keeps_nested_applications`, `a_duplicate_target_name_is_refused`, `a_missing_source_style_is_refused`, `a_character_style_rename_keeps_applied_references`, `renaming_in_a_non_editable_document_keeps_the_content`) to a new `crates/pictura-render/src/named_style/tests.rs`; verify the tests exist with `rg -n 'fn (renaming_an_applied_paragraph_style_keeps_nested_applications|a_duplicate_target_name_is_refused|a_missing_source_style_is_refused|a_character_style_rename_keeps_applied_references|renaming_in_a_non_editable_document_keeps_the_content)' crates/pictura-render/src/named_style/tests.rs` (expected not to compile until task 2.2).
- [x] 2.2 Add `crates/pictura-render/src/named_style.rs` implementing `edit_named_style` (validated sheet edit, applied-reference rewrite with group recursion, re-resolution guarded by `type_layers_editable` and preserving manual overrides), export it from `lib.rs`, and declare `#[cfg(test)] mod tests;` so the task 2.1 tests compile and run (or inline them); verify `cargo test -p pictura-render --lib named_style` reports at least one test (not merely exit 0) and `cargo clippy -p pictura-render --all-targets -- -D warnings` passes.
- [x] 2.3 Confirm the operation performs no composite or history work of its own, leaving history to the caller; verify a render test that a refused edit leaves layer pointers, type specs and the composite byte-identical.

## 3. History adapter (pictura-app)

- [x] 3.1 Add `renaming_an_applied_paragraph_style_records_one_undoable_state` and a refused-edit-records-nothing case to `crates/pictura-app/src/history/tests.rs`; verify the test exists with `rg -n 'fn renaming_an_applied_paragraph_style_records_one_undoable_state' crates/pictura-app/src/history/tests.rs` (expected not to compile until task 3.2).
- [x] 3.2 Make `history::edit_character_style`, `history::rename_character_style` and the re-shaped `history::edit_paragraph_style(doc, from, to, character, paragraph)` delegate to `pictura_render::edit_named_style` (create/delete unchanged), update the `crates/pictura-app/src/history.rs` re-exports, and pass `name` twice at the live-preview caller in `type_tools/imp.rs`; verify `cargo test -p pictura-app --lib history::tests` passes, including `renaming_an_applied_style_keeps_it_applied` and `a_style_operation_records_one_undoable_state_each`.

## 4. Bridge and panel adapters

- [x] 4.1 Add the target name to `type_edit_paragraph_style` in `crates/pictura-app/src/cxxqt_object/type_tools.rs`, and route both `type_edit_character_style` and `type_edit_paragraph_style` through one `pictura_render::edit_named_style` call inside `commit_style` in `type_tools/imp.rs`, leaving previews on the same-name path; verify `cargo clippy -p pictura-app --all-targets -- -D warnings` and the existing `pictura-app` library tests pass.
- [x] 4.2 Add the target-name argument to `type_api::editParagraphStyle` in `crates/pictura-app/cpp/api/type_api.{h,cpp}` and replace the create/delete sequence in `ParagraphStylesPanel::editStyle` with one call that returns the result and, on failure, re-previews the opening attributes before returning false; verify the CMake build succeeds with `bash scripts/with-build-lock.sh -- cmake --build build --parallel`.
- [x] 4.3 In `CharacterStylesPanel::editStyle`, on an accepted edit the operation refuses, re-preview the opening family and character attributes under the original name (the same path Cancel uses) before returning false; verify the CMake build succeeds with `bash scripts/with-build-lock.sh -- cmake --build build --parallel`.

## 5. Qt Test coverage

- [x] 5.1 Add `renameKeepsApplied`, `renameAndEditAreOneUndo`, `renameToExistingIsRefused` and `renamingBasicParagraphIsRefused` to `crates/pictura-app/cpp/tests/tst_paragraph_styles_panel.cpp`, asserting applied references, one history state, and unchanged state on refusal; verify `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_paragraph_styles_panel$' --output-on-failure`.
- [x] 5.2 Extend `tst_paragraph_styles_panel::createAndDelete` with an undo (and redo) assertion that restores the deleted style, covering the "Deleting another style removes it" scenario; verify `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_paragraph_styles_panel$' --output-on-failure`.
- [x] 5.3 Extend `tst_character_styles_panel::renameKeepsApplied` with a one-history-state/undo assertion and add `tst_character_styles_panel::renameToExistingRestoresPreview` for a refused accept that restores the previewed attributes under the original name; verify `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_character_styles_panel$' --output-on-failure`.
- [x] 5.4 Keep `dialogPreviewAppliesAndCancelRestores`, `tst_character_styles_panel::renameKeepsApplied` and `tst_character_styles_panel::dialogPreviewRestoresOnCancel` passing after the panel change; verify `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R 'tst_(paragraph|character)_styles_panel' --output-on-failure`.

## 6. Integration verification

- [x] 6.1 Run `python3 scripts/check-test-plan.py` and `openspec validate text-style-document-op --strict` from the repository root and fix any reported gap.
- [ ] 6.2 Run `bash scripts/verify-fast.sh` from the repository root and fix any fmt, clippy, report, file-size, guard or OpenSpec failure.

## 7. Review fixes

- [x] 7.1 When editing `Basic Paragraph` under its own name, re-resolve every type layer that applies another character or paragraph style (those layers inherit it as their base) and leave un-styled layers alone; add `editing_basic_paragraph_reaches_the_layers_that_apply_a_style` and the matching spec scenario and test-plan row.
- [x] 7.2 Strengthen `renaming_an_applied_paragraph_style_keeps_nested_applications` and `a_character_style_rename_keeps_applied_references` to assert the rendered channel planes change and that a manual override survives the rename plus re-resolve (no lines added to `crates/pictura-app/src/history/tests.rs`).
- [x] 7.3 Register the original family before the paragraph panel's refusal re-preview, matching the character panel's restore path.
- [x] 7.4 Run `cargo fmt --all` and confirm `cargo fmt --all -- --check` exits 0 (verified without building).
