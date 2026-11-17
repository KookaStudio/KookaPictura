# Proposal

## Why

Issue #1268 follows #1214 (PR #1253). `Selection.data` is always a tiled
`Plane<u8>`, but `Plane` dereferences to `[u8]`: every consumer that takes a
`&[u8]` selection, indexes `sel.data[i]` or calls `to_vec()` builds one
canvas-sized contiguous copy (a counted materialization) and keeps it cached
beside the tiles. At 17000² that is 276 MiB per selection, held until the next
write. #1214 ported the selection operations, fills, wand and Quick Mask; the
clipboard, mask edits, healing, filters and adjustments limited to a
selection, and about 67 reads in `pictura-app` still materialize.

## What Changes

- **Engine selection parameters take a plane.** The clipboard family
  (`copy_layer`, `copy_merged*`, `clear_layer`, `paste_clip`,
  `coverage_bounds`, `plane_has_coverage`), Layer via Copy/Cut, Lift,
  `move_mask_selection_content`, Patch and Content-Aware Move take
  `&Plane<u8>` / `Option<&Plane<u8>>` and read rows or tiles.
- **Filters and adjustments limited to a selection** read the selection mask
  one row at a time when they blend the filtered result back and when they
  build the clip-sized coverage the Auto adjustments count.
- **pictura-select leftovers.** `fit_centered` reads two rows per output row
  and returns a tiled selection. Refine Edge stays dense (design.md).
- **App readers.** Every `pictura-app` read of a selection's coverage goes
  through the plane's row/tile API. Bridge exports that hand a whole selection
  to C++ keep their one output `Vec`, built by row reads, so no cached mirror
  is left behind.
- **App writers.** Writes through `DerefMut` (`sel.data[..] = ..`) use
  `write_row_from` / `write_rect_from`, so the selection stays tiled.
- **Guard.** A heap-guard test runs one operation per group on a tiled
  selection and asserts 0 materializations.

## Capabilities

### New Capabilities

- `tools/tiled-selection-readers`: consumers of a selection read its tiles or
  rows; no consumer materializes it; whole-selection exports keep one output
  buffer.

### Modified Capabilities

None. Results are byte-identical; the behaviour existing specs describe is
unchanged.

## Impact

- `pictura-render`: clipboard, move_content, via, mask_edit, filter blend.
- `pictura-paint`: healing patch and content-aware move.
- `pictura-select`: `fit_centered`.
- `pictura-app`: selection bridge exports, clipboard, healing, transform,
  align, refine, filter tools, Quick Mask, selection move, Match Color, Color
  Range.
- No new dependency, no document-format change, no golden change.
