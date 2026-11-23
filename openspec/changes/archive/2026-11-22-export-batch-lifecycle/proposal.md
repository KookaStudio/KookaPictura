# Proposal

## Why

`File ▸ Scripts ▸ Export Layers to Files…` and `File ▸ Scripts ▸ Layer Comps to
Files…` each own a private copy of the same batch policy: destination
validation, target parsing and output-path composition, directory preflight,
one-file-per-step iteration, completed-item progress, cancellation, view and
parent lifetime handling, and stop-on-failure reporting (#1394). The file writer
is already shared; only the lifecycle is duplicated. Because the lifecycle lives
in each caller, the comps exporter's cancellation and write-failure checks are
skipped, and the shared behaviour is not the test surface.

## What Changes

- Add one C++ module, `export_batch.{h,cpp}`, that owns the file-export batch
  lifecycle: destination validation, `"<key>\t<name>"` target parsing, output
  path composition from a caller-supplied suffix, directory preflight, ordered
  per-target iteration, completed-item progress, cancellation before the next
  write, view/parent lifetime, stop-on-failure, and failure reporting.
- The module exposes the live run state (`total`, `written`, `cancelled`,
  `failed`, `cancel()`, `progressPermille()`) so cancellation between files,
  view destruction, empty targets, and progress monotonicity are checked
  deterministically, without timer- or sleep-based tests.
- Export Layers and Layer Comps keep only their adaptation through one spec
  builder each (`layersExportBatchSpec`, `layerCompsExportBatchSpec`), shared by
  production and tests: their existing target text, writer, suffix function, and
  destination/empty-list/per-target wording. Both local `BatchState` structs and
  both step/cancellation closure protocols are deleted.
- Report the failing target when the view is destroyed mid-batch. The comps
  exporter currently warns `Could not write .` because it records no failing
  target; the shared failure formatter supplies the target's path for both
  exporters.
- Replace the comps exporter's skipped cancellation and write-failure checks
  with runnable Qt Test checks that build the production comps spec and replace
  only its `write`, plus a deterministic second-write failure.
- Preserve dialog behaviour, target selection and rendering, file naming,
  format support and suffixes, wording (destination, empty-list, and per-target,
  all adapter-supplied), source document/history/dirty state, and partial
  outputs. Format and suffix resolution stay with each exporter.

## Capabilities

### New Capabilities

- `interop/file-export-batch`: the shared lifecycle contract for file-per-target
  exports - preflight before any write, ordered completed items, monotonic
  completed-item progress, cancellation observed before the next write, view
  destruction aborting the batch, stop-on-failure with completed files retained
  and later targets untouched, and adapter-supplied destination, empty-list, and
  target failure text.

### Modified Capabilities

None. `interop/export-layers-to-files` and `interop/layer-comps-export` already
describe the observable results; this change only gives their common lifecycle
one owner and a test surface.

## Impact

- `crates/pictura-app/cpp/export_batch.h`, `export_batch.cpp` (new).
- `crates/pictura-app/cpp/export_layers_dialog.h`,
  `crates/pictura-app/cpp/export_layers_dialog.cpp`,
  `crates/pictura-app/cpp/layer_comps_export.h`, and
  `crates/pictura-app/cpp/layer_comps_export.cpp` (spec builders and the
  rewired exporters).
- `cmake/sources/documents.cmake` and
  `crates/pictura-app/cpp/tests/CMakeLists.txt` registration.
- `crates/pictura-app/cpp/tests/tst_export_batch.cpp` (new) and
  `crates/pictura-app/cpp/tests/tst_layer_comps_export.cpp` (two skips become
  real checks elsewhere).
- No Rust changes, no new dependency, no format or naming change. Data Sets as
  Files (`variables/imp/export.rs`) and Zoomify keep their own batch engines.
