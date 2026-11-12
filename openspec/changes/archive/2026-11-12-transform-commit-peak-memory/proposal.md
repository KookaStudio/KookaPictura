# Proposal

## Why

Issue #1290: on a memory-limited host, a Free Transform commit on a large
document is refused with "not enough memory", where Photoshop CS6 completes it.
A code audit (posted on the issue) found three problems:

- An integer move with no scale or rotation still resamples every channel into
  new dense buffers, even though the result is the source pixels.
- The resample buffers are allocated infallibly, so a commit that gets past the
  gate on an underestimate aborts the process instead of failing cleanly.
- The commit gate charges a move for four dense layer planes that a move never
  allocates.

This change is phases 1 and 2 of the plan on #1290. The later phases follow in
their own pull requests: tile-streamed resampling, a banded composite with a
scratch-aware gate, and progress for spilling commits.

## What Changes

- **Integer-move fast path.** An integer move with scale 1 and no rotation does
  not resample. The layer keeps its existing tiles under a shifted rect, and the
  mask shifts with it. The result is byte-identical to the resampled result for
  every interpolation except Bicubic Smoother. That kernel's B-spline softens
  the layer even at whole-pixel offsets, so it keeps the resample path and its
  current output.
- **Fallible resample buffers.** `resample_plane` builds its padded source and
  its output with `memory::try_plane`. The native-depth resampler reserves its
  output fallibly as well. A failed allocation makes the transform refuse and
  leaves the document unchanged.
- **Gate.** `commit_bytes` no longer charges layer planes for a quad that is the
  original rect moved by whole pixels.

## Capabilities

- **ADDED** `document/transform-commit-memory`

## Impact

- `pictura-render`: `layer_ops/transform.rs`, `layer_ops/transform_native.rs`.
- `pictura-app`: `impl_transform/group.rs` (`commit_bytes`).
