# Retouch commits scale with their region

## Why

Issue #1192 measured the release path of three retouch tools on a 17000×17000
document (`scripts/profile-tools.py --size 17000x17000`): Patch **42 916 ms**,
Spot Healing **18 320 ms**, Red Eye **2 745 ms**. A later 48 MP run still took
Patch 11 s and Spot Healing 10 s, while the region composite was only 51–93 ms.
The remaining cost is engine work, not compositing.

Two causes are visible in `pictura-paint`:

- `heal_layer` builds an `RgbaImage` of the **whole layer** for every heal,
  whatever the region. Red Eye's box and a small Patch selection pay for the
  entire document.
- The Poisson/Laplace solves run a fixed 96 Gauss-Seidel sweeps single-threaded
  over the whole working rectangle. Patch's 0.35×0.3 selection at 17000² is
  about 30 M pixels, so the release spends tens of billions of inner-loop
  operations on one core. Spot Healing's 200 px zigzag is about 7 M covered
  pixels inside a larger bounding box.

The heals are behavioural parity only (Adobe's solver is closed), so the solve
may change as long as the existing tolerance contracts hold.

## What Changes

- **Windowed heal.** `heal_layer` materializes only the layer pixels an
  operation can read, and writes back only its changed rectangle. Each caller
  supplies a conservative window: the region plus its margin and boundary, the
  clamped clone/search source, or the move's destination. A small heal no
  longer copies a document-sized layer.
- **Parallel red-black relaxation.** The 4-neighbour Poisson and Laplace sweeps
  update one colour at a time; a same-colour cell's neighbours are all the
  other colour, so rows run on Rayon without reading a cell another thread
  writes. The iteration count and over-relaxation stay as they are, and the
  result is deterministic.
- **Red Eye is box-local and parallel.** `red_eye_layer` materializes only its
  clipped box and processes its rows in parallel with the same per-pixel
  formula.
- **Selection bounds scan in parallel.** The document-sized mask's bounding box
  is found by parallel row scans, so a Patch or Content-Aware Move does not
  serialise over every document row before the heal starts.

## Capabilities

### New Capabilities

- `tools/retouch-commit-responsiveness`

### Modified Capabilities

None. The existing healing, Patch, Red Eye and Content-Aware Move behaviour is
unchanged; this change adds the cost contract those capabilities rely on.

## Impact

- `pictura-paint`: `healing/layer.rs` (windowed materialization and write-back),
  `healing/mod.rs` (red-black relaxation), `healing/red_eye.rs` (parallel box),
  `healing/patch.rs` (parallel bounds), `healing/stroke.rs` and
  `healing/content_move.rs` (windows).
- `scripts/profile-tools.py` already drives `patch`, `spot-healing` and
  `red-eye`; no harness change is needed.
