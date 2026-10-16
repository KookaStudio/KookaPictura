# Proposal: reduce-noise

## Why

`Filter ▸ Noise ▸ Reduce Noise` is a disabled stub in `cpp/command_tree.cpp`;
there is no kernel and no photorust source. CS6's Reduce Noise reduces luminance
and colour noise while preserving edges, with a Basic mode (one Strength and
Preserve Details pair for the whole image) and an Advanced mode (per-channel
correction). Issue #343 asks for it.

## What Changes

- Add `ReduceNoiseParams` / `ChannelDetail` and `Filter::ReduceNoise`, with an
  edge-preserving bilateral denoiser per colour plane in `noise/reduce.rs`. A
  chroma pass blurs colour while holding luminance, an optional unsharp pass
  restores sharpness, and an optional 8×8 deblock removes JPEG blockiness.
  Deterministic, no RNG; alpha is left bit-identical.
- Add the `reduce-noise` kind (twelve slots) and a dedicated
  `ReduceNoiseDialog` with Basic / Advanced pages and a Channel menu.
- The menu leaf stops being a stub.

## Capabilities

### Modified Capabilities

- `imaging/noise-filters`: a new Reduce Noise requirement.
- `imaging/filter-app-ui`: the `reduce-noise` kind and its dialog.

## Impact

- `pictura-filters` (`noise`, `Filter`, `ChannelDetail`, `ReduceNoiseParams`),
  `pictura-app` (`filter_map.rs`, `filter_commands.cpp`,
  `reduce_noise_dialog.{h,cpp}`). No new dependency.
- **Oracle:** none. Adobe's kernel is closed and ImageMagick has no matching
  operator; covered by property tests and a kernel digest.
