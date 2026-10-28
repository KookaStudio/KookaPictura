# Design

## Context

See `proposal.md` for motivation and
`specs/document/history-retention/spec.md` for the contract.

`pictura_core::Plane` is already a refcounted, copy-on-write slice carrying a
**stamp**: a clone keeps the stamp, every mutable access clears it, and only the
history sets one. Two planes with the same nonzero stamp hold the same bytes.
`stamp_agreed` (`history/planes.rs`) is the existing seam that keeps the live
document and the history's private current equal and their tracked planes
unshared after a tile diff.

The only copy-on-write exception for tracked planes is structural capture:
`capture_live` takes the `diff_into`-failed branch, stores the old current as a
full anchor, and calls `private_copy` for the new current because the live
document must not share a tracked plane with history. `restore_live` has the
same exception when an anchor is crossed: it installs a private copy of the
target into the live document.

## Goals / Non-Goals

**Goals:**

- Remove the whole-document copies from structural capture and cross-anchor
  restore without changing a single stored byte.
- Keep the live↔history tracked-plane isolation invariant; tracked-plane
  sharing is allowed only between history states.
- Prove the retention bound with a normal unit test and measure time and memory
  with an ignored profile.

**Non-Goals:**

- Snapshot creation (`detached_copy`) still copies the document.
- Crop and canvas-size captures still copy planes whose bytes change; sharing
  there needs tile-level plane storage.
- Native-depth `Samples` are `Vec`s, so `Document::clone` still copies them for
  16/32-bit documents. The profile targets 8-bit RGB.
- `SmartObject.filter_mask.data` is a live `Plane` that `each_plane` does not
  traverse, so it remains shared with history and is outside this invariant;
  extending traversal would make smart-filter-mask edits delta-tracked and is a
  separate change.
- No timing assertions in the ignored profile; timings are printed, and the
  deterministic byte bound is asserted by a normal test.

## Decisions

### One helper, two directions: reuse a donor's planes by stamp

Chosen: `planes::shared_copy(source, donor)` builds a document with `source`'s
metadata and traverses its tracked planes in the canonical order. For each
tracked plane it takes `donor`'s plane when the two carry the same nonzero stamp
and the donor plane does not share the source plane's allocation; otherwise it
copies the source plane. It then calls the existing `stamp_agreed` so the result
and `source` carry equal stamps and share no tracked-plane allocation.

- Structural capture: `source = live`, `donor = old current`; the result becomes
  the new private current and may share planes with the full anchors of earlier
  segments.
- Full restore: `source = target`, `donor = live`; the result becomes the live
  document and keeps the live document's own tracked planes for everything the
  target did not change.

Both directions have the same contract: metadata from `source`, plane payload
from `donor` where bytes are proven equal by stamp, no tracked-plane share with
`source`. Using one helper avoids two subtly different stamp-agreement loops.

Alternatives considered: matching by index (breaks on reorder), comparing bytes
for unstamped planes (turns O(changed) into O(document)), and reusing the live
plane directly (violates the property that live's next write is in place).

### Build the new current before `fill_anchor`

`capture_live` must harvest the old current's planes *before* `fill_anchor`
walks it back to its segment anchor. Afterwards those planes have been reverted
and forked, and the cursor state's pixels are no longer materialized. The walk
then forks exactly the planes the segment's deltas touch, because those are the
ones now shared with the new current; every other plane stays shared. The
walk forks any plane it touches, so a plane shared outside the segment is
never mutated in place.

### Restore keeps the live document's tracked planes

On a cross-anchor restore, the live document's tracked planes are already
unshared and stamped. `shared_copy(target, live)` keeps them wherever the
target's stamp matches and copies only the target's changed tracked planes.
Undoing a New Layer on a 20-layer document therefore copies no layer pixels at
all, only the metadata and any tracked plane the target actually changes.

### Retained bytes count allocations, not planes

`retained_plane_bytes()` collects the tracked planes of every retained
document (states, current, pinned brush source, named snapshots) and sums one
length per distinct allocation, using `Plane::shares` for identity. The
test-only helper is O(number of planes²) with pointer comparisons, which is
negligible at history scale. It is `#[cfg(test)]`.

### Verification

- A normal test asserts the retention bound after twenty New Layer captures and
  that live shares no tracked plane with any full state.
- A focused test asserts that undoing an add keeps the live base layers' and
  composite's allocations and leaves live's tracked planes unshared.
- The existing `live_restores_match_a_full_snapshot_history` model test gains
  reorder steps interleaved with add, delete, and paint, and keeps its
  full-reference byte equality and live-isolation checks.
- A duplicate-layer test covers two live planes that carry one stamp.
- `large_document_structural_edit_profile` (ignored) starts with twenty
  full-canvas layers at 6000×4000, measures twenty New Layer captures, and
  prints the issue's targets (20 ms add+capture, 30 ms undo, 5 GB retained)
  beside the measured add+capture and undo times and retained bytes.

## Risks / Trade-offs

- **Duplicate Layer gives two live planes one stamp.** Matching by stamp maps
  both target planes to one donor plane, so the result shares one allocation
  between two planes. The first reuse keeps the stamp; every later reuse is
  given a fresh stamp, and `stamp_agreed` settles a matching fresh stamp on the
  live plane, so the allocation stays shared while `planes_moved` keeps its
  unique-stamp map. Covered by
  `structural_capture_with_a_duplicated_layer_round_trips` and
  `a_capture_after_a_duplicate_stays_a_delta`.
- **The first edit after a capture forks the touched plane in history.** Work
  moves from the structural capture (every plane) to the first paint on each
  changed plane (one plane each), which is strictly less in total and bounded by
  what changed.
- **`retained_plane_bytes` is O(n²).** It is test-only; a pointer-identity
  accessor in `pictura-core` would be API surface for no benefit.
- **Timing varies by machine.** The profile is print-only, matching the other
  print-only profiles; the deterministic unit test owns the pass/fail bound.
- **A plane whose stamp is 0 is always copied.** This is deliberate: a write
  clears the stamp, and comparing bytes would reintroduce whole-document scans
  on structural captures.
