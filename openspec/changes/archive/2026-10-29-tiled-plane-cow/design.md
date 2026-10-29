# Design

## Context

See `proposal.md` — Why. Current constraints that shape the approach:

- `Plane<T>` is `Arc<[T]>` + a `u64` stamp + an `AtomicU64` ident, and derefs to
  `[T]` (`crates/pictura-core/src/plane.rs`). Any mutable access clears both and
  `Arc::make_mut` copies the whole plane on the first write after a clone.
- `Plane` stores no width. Width comes from the call site: `each_plane`
  (`crates/pictura-app/src/history/planes.rs`) derives it from `doc.width`,
  `layer.rect`, or the mask rect. The composite plane is the channel blocks
  stacked vertically in one flat buffer.
- History keeps `private_copy` of every tracked plane from open and diffs 64×64
  tiles against it (`history/planes.rs`, `history.rs`).
- The compositor is already band-based: `composite_region::BAND == 64`, and the
  history delta tile is `TILE == 64`.
- `#1012` (COW samples/selections) and `#1013` (dirty-rect effect passes) are
  unmerged; this change touches neither and composes with both.
- `#1014` tracks this change.

## Goals / Non-Goals

**Goals:** no tracked-plane pixel copy at document open; a capture scans only
the bands that changed; a stroke write forks only the bands it touches; every
cold contiguous reader keeps working.

**Non-Goals:** replacing the stroke's own pre-stroke tile save; per-band
stamps/idents and per-tile render-cache keys; GPU resident keys that survive
mid-stroke; 2-D tiles; non-u8 history planes (they wait for `#1012`); any change
to the document bytes, history order, depth, labels, damage reporting, or the
public `History` API.

## Decisions

**D1. A band is 64 rows of the plane's width, not a 64×64 tile.**
A 64×64 tile is 64 disjoint runs in a row-major plane and cannot be one
`(Arc, Range)` handle; a band can. 64 matches both `composite_region::BAND` and
the history `TILE`. *Alternative:* 2-D tiles — needs a tile grid in `Plane`,
owned per-tile allocations (so fracture copies), and stride plumbing at every
access. Rejected for this change.

**D2. A fractured plane owns one base allocation and a band vector.**
`Plane<T>` gains `base: Arc<[T]>`, `bands: Vec<Band<T>>`, and a recorded width.
`Band<T>` is either `Shared(Range<usize>)` into `base` or `Owned(Arc<[T]>)`, a
forked copy of that band. A flat plane has no bands and reads `base` directly.
Cloning a plane clones the band vec and bumps `base`; it copies no pixels.
`fracture(width)` records the width and builds `Shared` ranges without copying
pixels. `write_band(i)` forks only band `i`: it writes in place when the band is
`Owned` with `Arc::strong_count == 1`, or when the band is `Shared` and
`Arc::strong_count(base) == 1`; otherwise it copies that band's range into an
`Owned` band. *Alternative:* a per-band `strong_count` against one shared base
Arc — a shared base is owned by every band and every clone, so it never proves
band uniqueness; rejected.

**D3. Keep the contiguous-read bridge with a cached materialization.**
`as_slice()` returns the base allocation while no band has diverged; after a
divergence it materializes one contiguous `Arc<[T]>` in a `OnceLock` and returns
that. Every `&mut` path replaces the `OnceLock` with a fresh empty one, so the
cache cannot go stale. The materialization counter is `pub` and always compiled,
so `pictura-app` tests and the ignored profile can read it.
*Alternative:* remove `Deref` and migrate all 276 read sites now — too large for
this change; the bridge keeps cold readers compiling.

**D4. Whole-plane writes flatten.**
`as_mut_slice()` and `DerefMut` materialize one contiguous flat allocation,
replace the band view with it, and are documented as a one-copy operation.
Whole-plane operations — fill, filters, and a GPU full upload — may use this
path. Region and row writes go through `write_band` / `row_mut` and fork only
the touched bands. *Alternative:* keep the band view and write each band
separately — every whole-plane consumer would need a band loop, and a fill
touches every band anyway.

**D5. Add a band-aware writer and migrate the hot stroke write path.**
`write_band(i)` and `row_mut(y)` are the region/row writer. The stroke's
`plane_slices` and dab loops in `crates/pictura-paint/src/stroke/raster.rs` and
the region apply in `crates/pictura-paint/src/stroke.rs` (the
`dst[k] = Some(c.data.as_mut_slice())` path) use it, so a stroke forks only the
bands it touches. A parallel dab asks for the mutable slices of its touched
bands before splitting them across tasks; untouched bands stay shared.
`crates/pictura-paint/src/fill.rs` and the filter family may stay whole-plane
and flatten. *Alternative:* defer the stroke migration — leaves the hot path
materializing a layer per dab, which is worse than today. Rejected.

**D6. Read rows on the hot paths; keep the contiguous bridge for cold readers.**
`composite::channel_plane` exposes the plane, and the per-pixel painter
(`composite_rows::composite_pixels`), the GPU source assembly
(`gpu/assemble.rs`), the route/existence checks, the composite patch and crop
helpers, and the stroke's pre-stroke save read rows through `Plane::row`
instead of materializing a banded plane. The stroke profile shows zero
materializations. Filters, effect mattes, and matting still read whole planes
through `as_slice`; a banded plane they read materializes once, and a following
write adopts that view as the base, so a capture after such a read scans that
plane fully. Untouched planes still skip every band.
*Alternative:* migrate every remaining whole-plane reader in this change — the
`channel` signature is used across filters, matting, and type layers, so that
migration can land after the storage change.

**D7. Plane-level stamp and ident stay; no per-band stamps yet.**
"Changed since the history state" is answered by comparing band allocations
(`Arc::ptr_eq`), which is exact and free. `shares(a, b)` means every band is the
same allocation: the same base with no band diverged. `shares_band(i, other)`
reports whether band `i` matches the corresponding band of `other`. Stamps and
idents remain plane-level and are cleared by any write, preserving
`known_equal`, `planes_moved`, and the existing cache keys. *Alternative:* a
per-band stamp vector now — state with no consumer in this change.

**D8. History shares bands; capture and restore move bands, not planes.**
`capture_live` fractures each tracked live plane (the width is already available
in `each_plane`) before sharing, and the current state holds the same handles.
`diff_into` skips bands whose allocation matches, tile-diffs only the changed
bands, and `write_bands` writes the changed tiles into the band they fall in.
`restore_live` writes tiles into the live bands, forking only touched bands.
`shared_copy` and `stamp_agreed` are band-aware: a plane whose bands are shared
is not copied, and only unshared bands or planes are copied. A stroke writes
through the band writer, so a capture scans only the touched bands.
*Alternative:* keep `private_copy` and share only at structural captures (what
`#973` did) — does not remove the open copy. *Alternative:* drop the
materialized cursor state entirely and rebuild any state by replaying at most
20 deltas — rejected because every edit path would then have to declare its
dirty rect at capture time, and the replay would move the cost onto undo.

**D9. The open path keeps stamping both sides.**
`capture_live` keeps calling the open path, but the fresh segment replaces
`private_copy` with a band-shared current state. It still assigns fresh stamps
to both the live document and the history cursor state, so `planes_moved`,
`known_equal`, and the GPU resident key keep working. A banded plane that is
never written never allocates again. `retained_plane_bytes` counts each band
allocation once and the base allocation once without materializing; the
inherent `len()` and `is_empty()` on `Plane` keep `each_plane`,
`retained_plane_bytes`, and `gpu/resident.rs` from materializing through
`Deref`.

## Risks / Trade-offs

- **A missed hot reader materializes per frame** → the materialized view is
  adopted as the base on the next write, so a stroke materializes a painted
  plane once rather than per dab; the counter is asserted zero across a capture
  and printed by the profile.
- **A contiguous read between a write and a capture drops that plane's band
  sharing** → the next write adopts the materialized view as the base, so the
  capture scans that plane fully; untouched planes still skip their bands.
  Migrating the compositor reader removes this cost.
- **The materialization cache doubles a banded plane's bytes while it exists** →
  it is invalidated on the next write and only cold readers (save, export,
  oracle, tests) are expected to use it.
- **Band granularity is 256 KB at 4000 wide, not 4 KB** → accepted: the win is
  sharing, not copy granularity. 2-D tiles are a follow-up.
- **A band write costs one atomic uniqueness check** → a band is forked only
  when it is not uniquely owned; a unique band writes in place.
- **The composite plane stacks channels, so a band can straddle a channel
  boundary** → bands are flat row chunks; `Damage::rows` already maps a
  composite tile across a channel boundary.
- **`stroke.rs` is 1177 LOC against the 1200 cap** → the band-writer migration
  and the region apply get a split task so the file keeps headroom.
- **`composite.rs` is 959 LOC against the 1200 cap** → the row/band view and its
  readers get a split task so the file keeps headroom.
- **`#1012` may change `Samples` to `Plane<u16>`/`Plane<f32>`** → the band
  storage is generic over `T`; non-u8 history planes wait for `#1012` rather
  than depending on it.

## Migration Plan

1. Plane storage and its tests (no behavior change).
2. History sharing and its tests (the memory win).
3. Band-aware stroke writer and the profile (protects the win); the compositor
   reader migration is a follow-up.
4. Run targeted checks, archive the change, then run the fast gate; no on-disk
   format change, so rollback is a revert.

## Open Questions

- Whether to expose the materialization counter through `PICTURA_METRICS` can be
  decided when the counter exists.
