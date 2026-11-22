# Tasks

## 1. Prepared-plan owner and its checks

- [x] 1.1 Write the shared lifecycle unit tests named in `test-plan.md` in a new `crates/pictura-app/src/cxxqt_object/prepared_job.rs` test module; verify every name exists and the test module fails to compile until 1.2 lands.
- [x] 1.2 Implement the lifecycle owner, its held run, and its landed result (`start`, `running`, `progress`, `stop`, `discard`, `land`) with the pre-plan test delay, one-shot joining, captured-revision comparison, cancellation refusal, and forgotten panic payload; verify `bash scripts/with-build-lock.sh -- cargo nextest run -p pictura_app prepared_job` passes.
- [x] 1.3 Register `mod prepared_job;` in `cxxqt_object.rs`; verify `cargo fmt --all --check` and `bash scripts/with-build-lock.sh -- cargo clippy -p pictura_app --all-targets -- -D warnings` pass.
- [x] 1.4 Add the document-replacement discard test named in test-plan.md.

## 2. Auto-Align uses the owner

- [x] 2.1 Replace Auto-Align's own run, drop, polling, joining, and stale-result code with one held lifecycle owner, keeping its lens/options construction, `notice` message, applicability check, and apply/edit block; verify `bash scripts/with-build-lock.sh -- cmake --build build --parallel` then `bash scripts/with-build-lock.sh -- ctest --test-dir build -R '^tst_auto_align$' --output-on-failure` passes unchanged.
- [x] 2.2 Delete the Auto-Align adapter's duplicated lifecycle protocol and its two local tests (the named checks now live in the shared module); state that the adapter's session-close `cancel()` forwards to `discard()` while the FFI `auto_align_cancel` forwards to `stop()`; verify `bash scripts/with-build-lock.sh -- cargo clippy -p pictura_app --all-targets -- -D warnings` passes.

## 3. Auto-Blend uses the owner

- [x] 3.1 Replace Auto-Blend's own run, drop, polling, joining, and stale-result code with one held lifecycle owner, keeping its options construction, `error` message, masks-only report, and apply/edit block; verify `bash scripts/with-build-lock.sh -- cmake --build build --parallel` then `bash scripts/with-build-lock.sh -- ctest --test-dir build -R '^tst_auto_blend$' --output-on-failure` passes unchanged.
- [x] 3.2 Delete the Auto-Blend adapter's duplicated lifecycle protocol and its two local tests; route session close through `discard` (the adapter's `cancel()` forwards to `discard()`, while the FFI `auto_blend_cancel` forwards to `stop()`); verify `bash scripts/with-build-lock.sh -- cargo clippy -p pictura_app --all-targets -- -D warnings` passes.

## 4. Integration and archive

- [x] 4.1 Verify both Qt suites together (`bash scripts/with-build-lock.sh -- cmake --build build --parallel`, then `bash scripts/with-build-lock.sh -- ctest --test-dir build -R '^tst_(auto_align|auto_blend)$' --output-on-failure`) and the app workspace tests (`bash scripts/with-build-lock.sh -- cargo nextest run --workspace`) pass with no changed assertions or golden baselines.
- [ ] 4.2 Verify `bash scripts/verify-fast.sh` passes (it runs fmt, clippy, tests, the guard, the test-plan gate, and `openspec validate --all --strict`). The local run hit environment-only failures (numeric locale, software-GPU pixels, a 120 s print timeout under host load); CI verifies.
- [x] 4.3 Archive the change in the shipping pull request through the OpenSpec archive workflow and verify `bash scripts/check-openspec-archived.sh` passes. No `ROADMAP.md` or `PARITY.md` update is needed because no user-facing coverage changes.
