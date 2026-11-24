# Proposal

## Why

Cross-document selection transfers currently broker dense pixels through the shell: three C++ adapters export a canvas-sized `Vec<u8>` from the source, hand `(bytes, width, height)` across the bridge, and Rust rebuilds a second dense plane in the destination. For a 17000×17000 document, two such planes total 578,000,000 bytes (551.22 MiB). That is a size calculation, not a measured clean-master baseline. A live selection or alpha channel's tiled `Selection` is materialized twice. Colour rows are derived into a tiled plane in the transfer owner. The same-document paths, Duplicate Channel's default, the Channels panel's own drop, and a colour-row Load, pay the same cost, and Save Selection exports a plane it then never uses.

## What Changes

- Introduce one transfer owner in Rust that acquires a source document's coverage (live selection, alpha channel, raster mask, or Channels-panel colour row) as the engine's `Selection` and applies it to a destination (new channel, existing channel or mask, selection, or new document).
- Same-document transfers acquire in place, so the shell never exports and re-imports its own document.
- Cross-document transfers pass the source view to the owner instead of a dense buffer; a tiled source shares tile storage with the destination wherever the destination output is the coverage itself.
- Remove the acquisition-plus-byte-handoff protocol from `frame_selection_dialogs.cpp`, `file_drop_router.cpp`, and `panels/channels_panel.cpp`; each adapter supplies only source, destination, and user intent.
- Stop exporting an unused coverage plane on the same-document Save Selection path.
- Add a heap guard that separates necessary destination work from temporary transport; a materialization-count-only assertion is insufficient because the current export reads tiles without incrementing that counter.
- Keep the existing behavioural contracts unchanged: same-size destination rule for the dialogs, fit-and-centre bilinear placement for drops, inversion and combine modes, channel-cap and invalid-target refusals, source immutability, and one destination history state on success.

## Capabilities

### New Capabilities

- `tools/selection-coverage-transfer`: one Rust owner for source coverage acquisition and destination application, tiled transfer with tile sharing where semantics permit, no canvas-sized dense bridge transport, and the allocation bound that proves it.

### Modified Capabilities

None. The observable selection, channel, duplicate, and drag behaviour stays as specified in `tools/selection-channels`, `ui/channels-panel`, and `tools/tiled-selection-readers`; this capability adds the transport and ownership contract those behaviours were implemented behind. Fit, combine, and history behaviour remains owned by `tools/selection-channels` / `ui/channels-panel`.

## Impact

- Rust: new `impl_selection/transfer.rs` core; bridge declarations in `impl_selection/bridge.rs` and thunks in `impl_selection/bridge/imp.rs`; `impl_selection/exports.rs` keeps only the three readers and imports the row helper from `channel_rows` after the split.
- C++ shell: `frame_selection_dialogs.cpp`, `file_drop_router.cpp`, `panels/channels_panel.cpp` (call sites only; generated headers stay inside `cpp/api/`).
- Tests: existing `tst_selection_save_load`, `tst_channels_panel`, `tst_channel_options`, `tst_mask_edit`, and `tst_calculations` coverage preserved; new Rust tile-sharing unit tests, heap-guard tests in the memory profile, and frame-driven Save/Load tests.
- No new dependency, no document-format change, no algorithm, interpolation, or history-policy change.
