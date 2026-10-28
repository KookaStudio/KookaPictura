# Tasks: smart-filter-apply

## Tests

- [x] Rust: every `pictura-codec`, `pictura-filters`, `pictura-render`, and `pictura-app` test named in `test-plan.md`
- [x] Qt Test: `tst_smart_filter_apply`, registered in `cpp/tests/CMakeLists.txt`

## Engine

- [x] `pictura-codec`: `pictura_filter` record (PTFL), `append_smart_filter`, `set_filter_mask_enabled`
- [x] `pictura-codec`: `author_filter_masks` at save time, hooked in `write.rs`
- [x] `pictura-filters`: `catalogue::smart_capable` and the resource guard test
- [x] `pictura-render`: `SmartFilterOp::Filters`, `smart_filter_record`, `apply_smart_filter`; fix the stale ponytail in `document_ops/smart_filters.rs`

## App

- [x] `filter_tools/smart_apply.rs`: target, refusal, preview, commit; Lens Correction moves onto it
- [x] Hooks: preview branch, `apply_filter_params`, `filter_commit_start_inner`, gallery commit and replay, `PictureView::apply_filter`; canvas proxy and exact preview decline
- [x] `pixel_filter_target` refuses a smart object
- [x] Menus: `filter_document_takes` honours `smart_refusal`; `filter_target_smart` greys Image > Adjustments

## Close-out

- [x] `docs/05-layers/smart-filters.md`, `docs/dev/doc-code-parity-gaps.md`, `ROADMAP.md` (separate `TASK-ALLOWS-DOCS` commit)
- [x] File the follow-up issues
- [x] `openspec validate --all --strict`, `scripts/check-test-plan.py`, then archive the change
