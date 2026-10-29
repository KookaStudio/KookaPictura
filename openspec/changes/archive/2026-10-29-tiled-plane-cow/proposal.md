# Proposal

Tracking issue: #1014.

## Why

A `Plane<T>` is one `Arc<[T]>` with one stamp and one ident. Any mutable access
clears both for the whole plane and the first write to a shared plane copies all
of it. History works around that by keeping a private copy of every tracked
plane from the moment a document opens (`private_copy`), so an 8-layer
4000×4000 RGBA document holds about 512 MB twice, and a capture scans every
plane tile by tile to find what changed. The copy exists only because a plane
cannot be shared at a granularity finer than the whole plane.

## What Changes

- Give `Plane<T>` band-granular copy-on-write storage: a flat plane keeps one
  contiguous allocation; a banded plane owns one `base: Arc<[T]>` and a
  `Vec<Band<T>>`, where each band is either a `Shared` range into `base` or an
  `Owned` forked copy of that band. Fracturing records the width and builds
  `Shared` ranges without copying pixels; cloning bumps `base` and clones the
  band vec; writing a band forks at most that band.
- Keep the contiguous read contract as a bridge: `as_slice()` returns the base
  allocation while no band has diverged, and materializes a cached contiguous
  view once a band has. Whole-plane mutable access (`as_mut_slice`, `DerefMut`)
  flattens to one contiguous allocation, documented as a one-copy operation.
- Add a band-aware writer (`write_band`, `row_mut`) and migrate the hot stroke
  write path to it: `crates/pictura-paint/src/stroke/raster.rs::plane_slices`
  and the dab loops, plus the region apply in `crates/pictura-paint/src/stroke.rs`,
  fork only the bands a stroke touches. `fill.rs` and filters may stay
  whole-plane and flatten.
- Deferred to a follow-up: move the compositor's hot plane reads from
  whole-plane slices to rows/bands. Until then a contiguous read materializes a
  diverged plane, the next write adopts that view as its base, and a capture
  after such a read scans that plane fully; untouched planes still skip bands.
- History stops keeping a private copy: at open, capture, and restore it shares
  band handles with the live document, compares bands by allocation, and reads
  or writes only the bands that changed. Opening still assigns fresh stamps to
  both the live document and the history cursor state.
- The band API is additive. `DerefMut` and `as_mut_slice` still exist and
  flatten; `fracture`, `bands`, `band`, `row`, `row_mut`, and `write_band` are
  new.

Out of scope, tracked as follow-ups: replacing the stroke's own pre-stroke tile
save, migrating the compositor's plane reads to rows/bands, per-band
stamps/idents and per-tile render-cache keys, GPU resident keys that survive
mid-stroke, and 2-D (non-band) tiles. Smart-object filter masks and native-depth
`Samples` remain outside the tracked set.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/document-model`: pixel planes become band-granular copy-on-write
  with a contiguous-read bridge and a flattening whole-plane write path,
  replacing the one-allocation/one-stamp plane contract.
- `document/in-place-history`: the current state shares bands with the live
  document instead of copying planes; capture and restore compare and move
  bands rather than whole planes.
- `document/history-retention`: shared tracked planes fork per band instead of
  per plane, and retained bytes are bounded by the bands a capture changed.

## Impact

- `crates/pictura-core/src/plane.rs`: banded storage, fracture, band/row access,
  contiguous-read bridge, band-level copy-on-write, inherent `len`/`is_empty`,
  public materialization counter.
- `crates/pictura-app/src/history/planes.rs`: band-aware sharing, per-band diff
  scan, band-wise tile apply, `private_copy` removed from the open path.
- `crates/pictura-app/src/history.rs`: capture and restore share bands; the
  structural-segment anchor path uses the same sharing.
- `crates/pictura-paint/src/stroke/raster.rs` and
  `crates/pictura-paint/src/stroke.rs`: the hot stroke write path forks touched
  bands; `fill.rs` and filters keep the flattening whole-plane path.
- `crates/pictura-render/src/gpu/resident.rs`: inherent `len` so the resident
  key does not materialize a banded plane.
- Tests: `pictura-core` plane tests, `pictura-app` history tests, the paint
  stroke suites, and existing `document_oracle` suites unchanged.
- No new dependencies. Document bytes, history order, labels, depth, damage
  reporting, and the public `History` API are unchanged.
