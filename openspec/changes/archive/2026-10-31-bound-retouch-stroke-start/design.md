# Design

## Context

See `proposal.md` — Why.

The paint engine already has the pieces this change needs:

- `Stroke::base_layer_rgba_doc` (`crates/pictura-paint/src/stroke.rs`) proves the
  straight-interleave pattern: it has a fast full-document path and a general
  offset/partial-rect path.
- The banded dab raster (`crates/pictura-paint/src/stroke/raster.rs`) shows how
  the crate cuts planes into disjoint row bands and runs them with rayon.
- `pictura_core::Plane` exposes `row(y, width)` (`plane.rs:317`), which reads one
  row without materializing a banded plane; `as_slice()` materializes the whole
  plane into a cached mirror.
- `rayon` is already a `pictura-paint` dependency, and `fill.rs` already builds
  row tasks with `rows.into_par_iter()`.
- `Document::clone` is a refcount bump after #1012, so `sample_scope`'s scoped
  clone is cheap; `composite_rgba` is already banded for large documents.

Constraints:

- `crates/pictura-paint/src/stroke.rs` is at 1185 lines against a 1200 code cap.
- `crates/pictura-paint/src/stroke/tests.rs` is at 1390 lines against a 1400
  test cap.
- `crates/pictura-app/src/cxxqt_object/tests_profiles.rs` is at 1162 lines
  against a 1200 code cap.
- `crates/pictura-app/src/cxxqt_object/paint_tools.rs` is at 1158 lines against
  the same cap.
- `crates/pictura-app/src/history/tests.rs` is at 1344 lines against a 1400
  test cap.

## Goals / Non-Goals

**Goals:**

- Every stroke source builder is one parallel row pass with the planes resolved
  once, byte-identical to today's serial output.
- Sample All Layers removes the document-sized surface build and crop.
- Pattern Stamp's setup does not scale with the document.
- Repeated History Brush / Art History Brush / Erase to History strokes do not
  rebuild the source until the resolved source or layer path changes.

**Non-Goals:**

- Making stroke start dab-sized. That needs tile-backed `PerDab.pixels` and
  tile-backed engine state; it is a follow-up (see Risks).
- Caching Clone Stamp sources.
- Touching the per-stroke GPU seed or the per-dab write-back in
  `apply_per_dab`.
- Changing `tools/in-place-stroke`.

## Decisions

### Resolve planes once, interleave whole rows, read rows through `Plane::row`

Each builder resolves the four channel indices once and then processes output
rows in parallel with `par_chunks_mut(width)`. A row reads each channel with
`Plane::row(y, width)` rather than `as_slice()`: a banded plane stays banded and
is not materialized into a second full-plane buffer. A zero width or height
returns an empty image before `par_chunks_mut(0)` can panic.

Missing channels differ by builder: `layer_rgba` defaults a missing colour
channel to `0` and a missing alpha to `255`, as today; `surface_from_composite`
defaults a missing channel, including alpha, to `0`; `layer_local` leaves
out-of-source pixels `[0, 0, 0, 0]`.

Alternatives: keeping `read_pixel`'s per-pixel `plane_index` scan (the current
cost); `as_slice()` (parallel but materializes up to four extra full planes).
`Plane::row` has one branch per row, not per pixel.

`layer_rgba` moves to a new `crates/pictura-paint/src/stroke/surface.rs` and is
re-exported `pub(crate)` from `stroke.rs`. The move keeps `stroke.rs` under its
cap; the rewrite is covered by the parity tests.

### Keep a serial oracle behind `#[cfg(test)]`

Each builder keeps a serial reference implementation in the same crate's tests.
Parity tests compare old and new output over the cases the spec names (offset
and partial rects, a Background without alpha, a source hanging off the
document, negative pattern origins, and a zero-size source). A golden image
would be slower to update and would not isolate which builder diverged.

### Fold the Sample All Layers crop into the composite build

`surface_from_composite_rect(buf, rect)` builds the layer-local image directly
from the planar composite, one parallel row pass. `composite_surface` in
`paint_tools.rs` calls it with the active layer's rectangle, which it resolves
from the target path before the stroke begins. `Stroke::begin_focus`,
`begin_smudge`, and `begin_mixer` take the sampled surface as layer-local and
stop calling `layer_local`; their docs and tests change with the signature.

The composite pass itself is unchanged and may still be document-sized when
`picture()` is not current. The fold removes the document-sized surface build
and crop, not the composite.

Alternative: keep the document-sized build and parallelize `layer_local` only.
That still allocates and writes a document-sized buffer per stroke, which is the
cost this change exists to remove.

### `StampSource` becomes an enum and shares its image through `Arc`

`StampSource` gains an `Image` / `Pattern` enum. `Image` keeps the existing
image, offset, and optional transform. `Pattern` holds the tile, origin, and the
document width and height; it reads
`tile.get((x - origin.0).rem_euclid(tile.width), (y - origin.1).rem_euclid(tile.height))`
in `at()` only inside the document bounds, returning `None` outside them. Both
hold `Arc<RgbaImage>` so a cached history surface can be handed to a stroke
without copying it.

`StampSource::new` and `transformed` take `impl Into<Arc<RgbaImage>>` so existing
callers that pass an owned `RgbaImage` stay source-compatible
(`Arc<RgbaImage>: From<RgbaImage>`). `StampSource::pattern(tile, origin, width,
height)` returns `Option<StampSource>` and returns `None` for a zero-size tile,
so it cannot call `rem_euclid(0)`; `begin_pattern_stamp` keeps its refusal. The
enum removes the `.transform` field, so the one test that reads it uses a
`transform()` accessor (or `matches!`).

The existing `tiled` helper is test-only now and is the oracle for the lazy
Pattern source.

Alternative: parallelize `tiled` and keep the document-sized image. That keeps
an O(document) allocation and write per Pattern Stamp stroke.

### History source cache keyed on the resolved source id

`Entry` already carries a stable `id` (`crates/pictura-app/src/history.rs:59-69`)
that is never reused. Give `Named` snapshots an `id: u64` from the same
`next_id` counter, and add `pinned_id: Option<u64>` to `History`; `pin_source`
stores the id of the source it pins alongside the snapshot.

`brush_source_key()` resolves `BrushSource` to that id: `Oldest` and `State(i)`
to `states[i].id`, `Snapshot(i)` to `snapshots[i].id`, and `Pinned` to
`pinned_id`. The cache is `RefCell<Option<((u64, String), Arc<RgbaImage>)>>`,
keyed on `(source id, layer path)`.

`brush_source_surface(path)` resolves the key first, checks the cache, and only
on a miss calls `brush_source_doc()` and `layer_surface`. Resolving before
`materialize` matters: a hit skips materializing the state as well as rebuilding
the surface.

A plain commit (a new history state) does not invalidate, because the source id
is unchanged. A different source id, a source that is no longer resolvable or
whose resolved id changes, a brush-source change, or a different layer path
does. A source kept as the pinned source when the stack drops it keeps the
reused surface. `drop_oldest` shifts `BrushSource::State(i)` to the same logical
entry, so the id stays stable; `Oldest` pins and stores the id. Jumps and
undo/redo move the cursor but do not change the resolved source id, so they do
not invalidate. No generation counter and no bumps.

Clone Stamp is deliberately not cached: its source is the live document at
stroke start, and every committed clone stroke changes that document, so a
document-revision key would either miss or return stale pixels. The Clone Source
overlay it could share with is not implemented (`doc-code-parity-gaps.md`,
TOOL-13).

### Leave `tools/in-place-stroke` unchanged

The existing requirement says stroke start must not grow with the document size.
Parallel builders still do — they are memory-bandwidth bound at 267 MP. Rather
than weaken the requirement, this change leaves it in place, records the gap
here, and tracks the tile-backed follow-up as a separate issue. The new
capability states the guarantee this change actually provides.

## Measurements

Retouch stroke-start `begin` on an 8000×6000 (48 MP) document, release, 16
cores, median of three warm runs. Baseline is the pre-change HEAD (`1f730219`)
with an equivalent profile built on the old serial builders; after is
`retouch_stroke_start_profile` on this branch.

| Tool | Before | After | Speedup |
| --- | ---: | ---: | ---: |
| Brush (control) | 0.06 ms | 0.06 ms | 1× |
| Dodge | 484 ms | 13.8 ms | 35× |
| Smudge + Sample All | 829 ms | 29.8 ms | 28× |
| Clone Stamp All Layers | 767 ms | 532 ms | 1.4× |
| History Brush | 534 ms | 15.6 ms | 34× |
| Pattern Stamp | 118 ms | 0.04 ms | ~3000× |

The Pattern Stamp's after figure is at timer resolution. Clone Stamp All Layers
is dominated by the scoped `composite_rgba` (~500 ms), which this change does
not touch; only its surface build shrank. The 267 MP `ips_first_dab` bound is
still not claimed (see Risks).

## Risks / Trade-offs

- **Still O(document), so `ips_first_dab` remains unmet for these tools** →
  Follow-up issue: tile-backed `PerDab.pixels`, tile-backed `ColorReplacer.done`
  and `ToneBrush.applied`, and a halo per dab for neighbour-reading engines.
- **A banded `Plane::row` call per row has a branch** → Negligible next to the
  per-pixel channel scan it replaces; the profile will confirm.
- **`Arc<RgbaImage>` ripples through `StampSource` callers and tests** →
  Constructors take `impl Into<Arc<RgbaImage>>`; tests construct `RgbaImage` and
  let it convert.
- **Cache staleness if a source id is reused or a source's pixels are rewritten
  under the same id** → Entry ids are never reused (`history.rs:60-62`) and a
  source's pixels are immutable, plus a test that a commit between strokes still
  hits.
- **File-size caps** → New `stroke/surface.rs`, `history/tests/cache.rs`, and
  `tests_profiles_retouch.rs` modules; `stroke.rs` shrinks by the moved helper.
- **More parallel code to keep byte-identical** → Every builder keeps its serial
  oracle and a parity test.

## Migration Plan

Internal only: no file-format, API, or user-visible behavior change. Rollback is
reverting the commit; the serial oracles make the old behavior available for
comparison during review.
