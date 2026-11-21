# Tasks

## 1. Tests

- [x] 1.1 Extend `tst_control_editing::exportWebpModes` to exercise the default
  options, and add `tst_control_editing::exportWebpRejectsBadOptions` for
  `quality` 101, `quality` -1, a fractional `quality`, a string `quality`, and a
  non-boolean `lossless`.
- [x] 1.2 Add `tst_export_as::webpQualityRange` for quality 0 and 100.

## 2. Code

- [x] 2.1 Validate `quality` (integer 0..=100) and `lossless` (boolean when
  present) in `ControlServer::methodExportWebp` before exporting.
- [x] 2.2 Set the Export As Quality range to 0..100.

## 3. Docs

- [x] 3.1 Update `docs/10-workflow-io/export-formats.md`,
  `docs/11-cross-cutting/agent-control.md`, and `docs/dev/STATE.md` for the
  shipped WebP lossy/lossless surface and `export_webp`.

## 4. Verify

- [x] 4.1 Run the focused Qt tests, `cargo fmt --all --check`,
  `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`, and
  archive the change. `cargo clippy -p pictura_app` was attempted but the
  cxx-qt build script failed to create its include symlink in the shared target
  directory; no Rust source changed in this audit.
