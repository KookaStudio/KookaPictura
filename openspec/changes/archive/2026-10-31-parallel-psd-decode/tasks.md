# Tasks

## 1. Test plan

- [x] 1.1 Write `test-plan.md` with one row per scenario in `specs/codec/psd-decode-parallelism/spec.md`, naming the tests this list adds, and verify `python3 scripts/check-test-plan.py` reports only the not-yet-written tests

## 2. Layer channel decode (`pictura-codec`)

- [x] 2.1 Add a `serial<T: Send>(f: impl FnOnce() -> T + Send) -> T` helper to `src/read/tests.rs` that installs `f` on a one-thread rayon pool (mirroring `write_cache::parallel_encoding_matches_serial`), and verify a throwaway `par_iter` test returns the same result under it
- [x] 2.2 Add `parallel_layer_decode_matches_serial`: write fixtures with `write_psd`/`write_psb` covering RLE, ZIP, ZIP-with-prediction, 16-bit, 32-bit, PSB, a user mask (-2), a real mask (-3), an extra color channel, and group dividers; build the depth-1 layered fixture by hand in `src/read/tests.rs` because `write_bitmap`/`output_depth` cannot emit it; read each through the parallel path and the one-thread pool and compare the `Document`s via `.expect(...)` (not `Result` equality, since `PsdError` is not `PartialEq`); verify the test passes
- [x] 2.3 Add `parallel_decode_reports_the_same_error` (a channel length exceeding the remaining bytes with the section length still fitting, truncated RLE count table, truncated scanline, short and long ZIP payloads) and `parallel_decode_reports_the_first_error_in_order` (two failing channels with distinguishable variants, earlier one wins); verify both pass
- [x] 2.4 Add `over_budget_channel_is_refused_before_decoding` in `tests/hostile_inputs.rs` (an over-budget later channel beside a malformed earlier one) and make the test allocator's thread-local `PEAK` observe worker threads (a global atomic max, or run the parse on a one-thread pool); verify the test passes
- [x] 2.5 Split `read_layer_info` into a serial planning phase (task list, `charge_channel` for decoded channels in record order, first-seen compression, total length), a `par_iter` decode phase in `read/decode.rs` that decodes each task from its own `Reader` slice with `Raw { offset, len }` and `Decode { offset, len, w, h }` variants, and an ordered reassembly phase that restores `r.pos` before the `info_end` check; verify 2.2 and 2.3 pass and `cargo nextest run -p pictura-codec` passes
- [x] 2.6 Add `layer_channel_tasks_record_their_own_offsets` after the split, asserting each recorded channel task's offset equals the running sum of the preceding declared lengths for a multi-layer fixture; verify the test passes
- [x] 2.7 Verify `cargo test -p pictura-codec --test hostile_inputs` passes, including every budget test listed under `pd_budget_existing` in `test-plan.md`

## 3. Merged RLE composite rows (`pictura-codec`)

- [x] 3.1 Add `parallel_rle_composite_matches_serial` at depth 8, 16, 32, and depth-1 Bitmap with one and with several channels, comparing the parallel and single-thread pools
- [x] 3.2 Add `merged_zip_composite_matches_serial` for ZIP and ZIP-with-prediction image data, leaving the deflate stream serial
- [x] 3.3 Rewrite `read_rle` to prefix-sum the scanline byte counts with checked offsets, decode each row into its own `par_chunks_mut` slice from the file-tail payload, and scan the collected row results in order; verify 3.1 and 3.2 pass
- [x] 3.4 Verify `cargo nextest run -p pictura-codec` and `cargo test -p pictura-codec --test depth_oracle` pass

## 4. ZIP-with-prediction inversion (`pictura-codec`)

- [x] 4.1 Add `parallel_prediction_matches_serial` comparing the parallel and single-thread pools at depth 8, 16, and 32
- [x] 4.2 Rewrite `undo_prediction` to process rows with `par_chunks_mut` at depth 8, 16, and 32, move the depth-32 unshuffle into the per-row closure so it no longer copies the whole buffer, and preserve the per-depth short-row, `width == 0`, and `rows` semantics; verify 4.1 passes
- [x] 4.3 Verify `cargo test -p pictura-codec --test depth_oracle`, `cargo test -p pictura-codec --test oracle`, and `cargo test -p pictura-codec --test hostile_inputs` pass

## 5. Integration and repository contracts

- [x] 5.1 Add the ignored `profile_psd_decode_stages` test in `src/read/tests.rs` (not `src/tests.rs`, which is at its allowlist ceiling) timing layer decode, merged RLE, inflate, prediction, and the serial total for a synthetic 8000×6000 30-layer document plus a 4×4 document, run `cargo test -p pictura-codec -- profile_psd_decode_stages --ignored --nocapture`, and record the numbers in the PR body
- [ ] 5.2 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`, and `bash scripts/verify-fast.sh`, and verify all pass, including the file-size and self-test-budget guards
- [x] 5.3 Update `ROADMAP.md` under "Engine and performance" with the shipped parallel PSD decode entry linking #1019, and verify no remaining `[ ]` item lacks an issue link
- [ ] 5.4 Archive the change with `openspec archive parallel-psd-decode`, then run `openspec validate --all --strict` and `bash scripts/verify-fast.sh`, and verify both pass
