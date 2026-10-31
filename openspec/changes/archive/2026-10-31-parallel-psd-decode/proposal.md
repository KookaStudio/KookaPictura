# Proposal

## Why

Opening a PSD already runs off the GUI thread, but the decode itself uses one
core: `read_layer_info` walks every layer and channel through a single
`Reader`, `read_rle` decodes the merged composite one scanline at a time, and
`undo_prediction` inverts a ZIP-with-prediction plane with one serial byte
loop. Every layer channel and every composite row is an independent stream, so
a 24 MP document leaves all but one core idle while the open spinner runs. The
codec already depends on rayon and uses it on the write side (`write_layers.rs`)
and for ICC conversion (`icc.rs`), so the decode path is the remaining gap.

## What Changes

- Split layer-channel reading into a serial planning phase and a parallel
  decode phase: walk the already-parsed layer records, charge the decode budget
  for every decoded channel in record order, record each channel's slice, then
  decode the slices with `par_iter` and reassemble in the original order, fixing
  the first-seen layer compression while reassembling in that order.
- Decode merged RLE composite rows in parallel using a prefix sum over the
  scanline byte-count table, writing each row into its own slice of the output
  plane.
- Invert ZIP-with-prediction in parallel: the per-row delta is independent at
  depth 8, 16, and 32, and the depth-32 byte-plane unshuffle is per row too, so
  both run per-row instead of over the whole buffer.
- Keep the merged ZIP stream itself serial: it is one deflate stream and
  splitting it needs a parallel inflate implementation and a new dependency.
- Keep output byte-identical to the serial decode, and keep the first error in
  record (or row) order, so a malformed file reports the same `PsdError`
  variant it reports today.
- Keep a test-selectable serial path (a one-thread rayon pool), mirroring the
  writer's `parallel_encoding_matches_serial`.

## Capabilities

### New Capabilities

- `codec/psd-decode-parallelism`: independent PSD/PSB layer channels, merged
  RLE rows, and ZIP-prediction rows decode in parallel while producing the same
  bytes and the same first error as the serial decode.

### Modified Capabilities

None. `codec/psd-codec` and `codec/untrusted-input-limits` keep their existing
requirements: the decoded document and the budget refusals are unchanged.

## Impact

- `crates/pictura-codec/src/read.rs` (`read_layer_info`), `read/decode.rs`
  (`read_rle`, channel decode entry point), `src/depth.rs`
  (`undo_prediction`, `unshuffle_32`).
- No new dependency: `rayon` is already a codec dependency and is already used
  by `pictura-core`, `pictura-render`, and `pictura-app`.
- No API, format, or on-disk change. Peak decode memory is unchanged for the
  retained channels; the transient peak for dropped folder/divider placeholder
  channels is bounded by the decode budget, not eliminated.
- One observable ordering change: the decode budget is charged for every decoded
  layer channel before any channel is decoded, so a budget refusal now precedes a
  decode error in an earlier channel. The refusal itself, its variant, and its
  scenario are unchanged.
- Out of scope: skipping the open-time recomposite when a merged composite is
  present (`compositing/composite-view` requires the open view to show
  `composite_rgba`, so that is a spec change), and parallel inflate.
