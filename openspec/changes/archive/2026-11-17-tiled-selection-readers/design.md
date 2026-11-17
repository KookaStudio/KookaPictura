# Design

## Context

`Plane<T>` implements `Deref<Target = [T]>`. On a tiled plane the deref
materializes the whole plane once (counted by `pictura_core::materializations`)
and caches the copy as a mirror until the next write. Since #1214 every
selection is tiled, so any consumer that receives `&sel.data` as `&[u8]`,
indexes it, or calls `to_vec()` pays one canvas-sized copy and keeps it alive.

Owner decisions (2026-11-16):

- **One PR, a commit per group.** One OpenSpec change covers every group.
- **Engine masks take a plane.** Functions that took a document-sized
  `&[u8]` / `Option<&[u8]>` selection take `&Plane<u8>` /
  `Option<&Plane<u8>>` and read rows or tiles inside their loops.
- **Exports keep their output.** Bridge functions that hand a whole selection
  to C++ (`selection_coverage_bytes`, `channel_coverage`, `color_row_coverage`,
  the clipboard's coverage) keep one output `Vec`, filled with
  `Plane::read_into`, which copies tiles without the counted mirror. The C++
  side is unchanged.
- **Writers are in scope.** `DerefMut` writes move to `write_row_from` /
  `write_rect_from`.

## Decisions

### Row reads, not per-pixel samples

Loops that index `coverage[y * w + x]` read one row span per output row
(`Plane::row_span`, borrowed on a flat plane, a short copy on a tiled one) and
index it. `Plane::sample` is kept for single-pixel lookups (the selection
probe, the filter tool's hit test). Bounds come from `coverage_bounds`, now a
row scan over the plane (`plane_coverage_bounds` merged into it).

### Auto adjustments keep a clip-sized slice

`pictura_adjust::apply_with_coverage*` takes a coverage slice parallel to the
work buffer: it is the size of the filter's clip, one plane of a buffer that
already exists, not a document-sized selection. It stays `Option<&[u8]>`; the
render filter fills it from selection rows instead of per-pixel derefs of the
mask. `filter::coverage` reads `Plane::sample`, and `write_back` reads one
mask row span per output row.

### Bounded work buffers stay

Healing builds an `f32` coverage over the selection bounds, Lift and the
clipboard build their bounds-sized clips: those are the algorithm's output or
work region, not a materialization of the selection, and stay.

### Tests

The selection is the same plane flat and tiled, so each ported function is
checked by running it on a tiled selection inside `assert_no_materialization`
and comparing with the same call on the flat copy (the pre-change dense path).
Existing tests keep their assertions; call sites that passed a `Vec<u8>` slice
now pass a flat plane built from the same bytes.

## Out of scope

- Refine Edge (`pictura_select::refine`): it builds whole-canvas `f64` and
  `bool` work arrays from the selection, eight times its size; reading the
  selection by rows would not bound it. It stays on the counted path, as
  #1274 left its work arrays (`ponytail:` in `refine.rs`).

- Layer masks and filter masks that are not selections (`LayerMask.data`
  readers in the compositor) — they are document planes covered by #1213.
- Making C++ consumers stream bands (the exports keep one output buffer).
