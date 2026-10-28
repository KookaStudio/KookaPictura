# Proposal

## Why

A structural edit (New Layer, Duplicate, Delete, Reorder) starts a new history
segment, and `capture_live` then deep-copies every tracked plane into the
private current state because the live document must not share a plane with
history. Twenty New Layers on a 6000×4000, 20-layer document retain about 58 GB
of history and make undo copy the whole document again. The planes that did not
change are already copy-on-write and stamped, so they can be shared between
history states at no cost.

## What Changes

- Build the private current state of a structural capture from the previous
  current state's planes, matched to the live planes by stamp, and copy only the
  planes that are new or changed. The first capture still copies the document.
- Install a restored state into the live document the same way: keep the live
  document's stamp-equal planes and copy only the planes the target changes,
  instead of replacing the whole document with a private copy.
- Keep the existing invariant that no tracked plane is shared between the live
  document and any history state; history states may share planes with each
  other, and a write forks through `Plane`'s copy-on-write.
- Add a test-only `retained_plane_bytes()` that counts each allocation once, and
  an ignored 6000×4000 profile for add/undo cost and retained memory.
- Snapshots (`detached_copy`), crop/canvas-size plane copies, and tiled storage
  remain as they are.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/history-retention`: structural anchors share every unchanged plane
  with the states around them, live restores keep the live document's own
  stamp-equal planes, and retained plane bytes scale with the planes a capture
  changes rather than with the document.

## Impact

- `crates/pictura-app/src/history/planes.rs`: a `shared_copy` helper that reuses
  a donor document's planes by stamp and copies the rest.
- `crates/pictura-app/src/history.rs`: the structural capture path and the
  full restore path use it; test-only `retained_plane_bytes`.
- `crates/pictura-app/src/history/tests.rs`: retention, live-isolation,
  structural restore, and model-test coverage.
- `crates/pictura-app/src/cxxqt_object/tests_profiles.rs`: the ignored profile.
- No new dependencies, no public API change, no behavior change to the
  document bytes, history order, labels, depth, or damage reporting.
