# Proposal

## Why

Issue [#807](https://github.com/KookaStudio/KookaPictura/issues/807) shipped a
lossy WebP export, but the merged surface still has gaps:

- `export_webp` passes `quality` and `lossless` straight through. A negative
  `quality` silently selects the lossless encoder, a value over 100 is accepted,
  and a non-boolean `lossless` is ignored. The capability spec requires quality
  0 through 100 and a boolean.
- The Export As Quality control starts at 1, so the codec's valid quality 0 is
  unreachable from the dialog.
- The long-form export and agent-control docs still describe WebP as
  lossless-only with no quality control, and omit `export_webp` from the Save
  permission row.

## What Changes

- Validate `export_webp` options before writing: `quality` must be an integer
  from 0 through 100 and `lossless`, when present, must be a boolean. A bad
  value answers `invalid_param` and writes nothing.
- Widen the Export As Quality control to 0 through 100, matching the engine and
  the control surface.
- Update the export-format, agent-control, and STATE docs to describe the lossy
  path, the Lossless control, and `export_webp`.

## Capabilities

### Modified Capabilities

- `interop/agentic-control`: `export_webp` rejects out-of-range or wrongly typed
  options.
- `interop/raster-export`: Export As WebP quality spans 0 through 100.

## Impact

- `crates/pictura-app/cpp/control_server_commands.cpp`
- `crates/pictura-app/cpp/export_as_dialog.cpp`
- `crates/pictura-app/cpp/tests/tst_control_editing.cpp`
- `crates/pictura-app/cpp/tests/tst_export_as.cpp`
- `docs/10-workflow-io/export-formats.md`, `docs/11-cross-cutting/agent-control.md`,
  and `docs/dev/STATE.md`
