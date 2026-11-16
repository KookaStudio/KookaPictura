# Tasks

## 1. Core edition model

- [x] 1.1 Add `pictura_core::Edition` and `Feature` with `from_env()`,
      `allows()` and `Feature::edition()`; verify the unit tests for parsing,
      the Extended default and the Timeline exception.
- [x] 1.2 Re-export `Edition` and `Feature` from `pictura_core`; verify the
      crate builds.

## 2. Catalogue policy and bridge

- [x] 2.1 Add `tool_edition()` and `command_edition()` to `pictura-script`,
      with the Count tool, 3D commands, Measurement Log and Count overlay marked
      Extended; verify the mapping tests.
- [x] 2.2 Carry `extended_only` on `ToolSpecRow` and `CommandSpecRow`, and add
      `edition_is_extended()` to the engine-tables bridge; verify the Rust
      bridge tests and a clean compile.
- [x] 2.3 Add `pictura-core` to `pictura-script`'s dependencies (internal path
      dependency, no new external crate).

## 3. Shell gate

- [x] 3.1 Add `toolAvailable()` and use it in the toolbox and `ToolController`
      for group membership, flyout entries, cycling and `setActiveTool`; verify
      the Standard/Extended toolbox tests.
- [x] 3.2 Skip Extended-only commands in `addDefaultCommands` before their
      separators are added; verify the Standard/Extended command tests.
- [x] 3.3 Add `tst_edition_gating::editionGating` covering both editions and
      the Timeline exception; verify the suite passes.

## 4. Documentation

- [x] 4.1 Tick the ROADMAP item and name `PICTURA_EDITION` and the gated
      surfaces; verify `ROADMAP.md` has no remaining `#650` unchecked row.
- [x] 4.2 Update `docs/README.md`, the `OVR-002` Rust mapping and the
      parity-gap register OVR-2/XC-10 rows; commit docs separately with
      `TASK-ALLOWS-DOCS`.

## 5. Verification

- [x] 5.1 Run `cargo fmt --all --check` and
      `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 5.2 Run `bash scripts/verify-fast.sh` and `openspec validate --all --strict`.
      The unified report reached 9772 passed / 141 skipped / 2 failed; both
      failures reproduce on master at 9d2eeef7 (`LargeDocumentTest::cancellingATransformEndsIt`
      and the host-specific `OpenSaveTest::rasterSaveKeepsFileInfo`).
- [x] 5.3 Archive the change with `openspec archive edition-gating --yes`.
