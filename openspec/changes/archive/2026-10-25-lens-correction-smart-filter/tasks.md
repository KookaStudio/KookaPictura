# Tasks: lens-correction-smart-filter

## Tests

- [x] Rust: every `pictura-codec`, `pictura-render`, and `pictura-app` test named in `test-plan.md`
- [x] Qt: the new `tst_lens_correction` cases named in `test-plan.md`

## Engine

- [x] `pictura-filters`: move the slots→`LensCorrectionParams` mapping out of the app bridge (`LensCorrectionParams::from_slots`, `SLOTS`)
- [x] `pictura-codec`: `lens_correction` module (id, name, keys, encode/decode, `attach_lens_correction_filter`), `lib.rs` exports
- [x] `pictura-render`: `SmartFilterOp::LensCorrection`, `document_ops::lens_correction` (`apply_lens_correction`, `lens_correction_slots`), `smart_source_buffer`, exports

## App

- [x] `pictura-app`: `filter_tools/lens_correction.rs` uses `from_slots`; the session reads the smart source and the stored slots; `lc_stored`
- [x] `pictura-app`: route a `lens-correction` commit on a smart object to the smart path (`filter_tools.rs`, `filter_tools/jobs.rs`)
- [x] C++: `lens_correction_dialog.cpp` prefills from `lc_stored`; `build.rs` if a bridge declaration changes

## Close-out

- [x] `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md` (FILT-10 closes)
- [x] `openspec validate --all --strict`, `scripts/check-test-plan.py`, then archive the change
