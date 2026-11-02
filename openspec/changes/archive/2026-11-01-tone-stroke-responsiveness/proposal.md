# Proposal

## Why

Dodge, Burn, and Sponge lag badly on very large images (#1129). On the 16507×16196
world map the first press took 0.5–1.7 s and a move at 1940 px took 140 ms, against
67 ms and 46 ms for the Brush. The toning tools never moved to the banded, parallel
path the Brush uses, and they break the in-place stroke requirement that a first
dab copies no layer plane.

## What Changes

- **Tone dab.** Rows run in parallel, the tip is built once per dab, and each row
  walks only the tip's chord. The per-pixel coverage record takes lazily zeroed
  memory.
- **Lazy pixels.** A toning stroke fills its working image a 64×64 tile at a time
  as dabs reach it, instead of copying the layer when it begins.
- **Banded write-back.** Every per-dab engine writes each dab's own rectangle
  through the plane bands. Blur, Sharpen, Smudge, Mixer, Color Replacement, Art
  History, and the Background Eraser share this step; their dab loops stay as they
  are.

## Capabilities

### New Capabilities

- `tools/tone-stroke-responsiveness`: bounded, byte-exact toning strokes and the
  per-dab write-back.

## Impact

- `crates/pictura-paint`: `tone.rs`, `stroke.rs`, `stroke/raster.rs`,
  `stroke/surface.rs`.
- `scripts/profile-tools.py` gains toning scenarios; `tests_profiles/retouch.rs`
  gains `tone_stroke_profile`.
- Tile-backed pixels for the other retouch engines stay with #1070.
