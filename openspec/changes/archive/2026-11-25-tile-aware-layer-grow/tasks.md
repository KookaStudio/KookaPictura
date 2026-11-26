# Tasks

## 1. Measure

- [x] 1.1 Time one stroke's phases at 21,000² (grow, dabs, record) and find the cost.
- [x] 1.2 Keep the breakdown as the ignored `long_stroke_phases_profile`.

## 2. Tile-aware grow

- [x] 2.1 Add `TiledPlane::padded` and `Plane::padded`: skip empty tiles, share tiles that land on a same-sized tile of the new grid, copy the rest.
- [x] 2.2 Route `padded_plane` through it, keeping the row copy for flat planes.
- [x] 2.3 Unit tests: equality with a row copy at aligned and unaligned offsets, one and two channels; full tiles shared on a right/down grow; empty tiles not written; a layer grow keeps its tile allocations.

## 3. Close out

- [x] 3.1 Build once; run fmt, clippy, the core and paint tests, and the profile; record the new stroke timings here.
  Measured with `long_stroke_phases_profile` (release, 21,000², one run each). Before: grow 1,709 ms on the first stroke (121 grows) and 6,735 ms on the last (32 grows); dabs about 175 ms; 15 strokes in 60.0 s. After: grow 106 ms on the first stroke and 52 ms on the last; dabs about 186 ms; record at most 8 ms; 15 strokes in 3.6 s.
- [x] 3.2 Update `docs/dev/STATE.md`; review `ROADMAP.md` and `PARITY.md` (no feature coverage moved; unchanged).
- [x] 3.3 Archive the change.
