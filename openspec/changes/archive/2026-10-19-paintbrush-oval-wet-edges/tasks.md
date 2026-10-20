# Tasks: paintbrush-oval-wet-edges

## 1. Engine

- [x] 1.1 Add `StrokeConfig::wet_edges` and the wet-edge transfer curve to
  the Paint stroke's coverage.
- [x] 1.2 Add `PaintMode::Multiply` to `blend_pixel`.

## 2. App

- [x] 2.1 Carry Wet Edges through `PaintTip`, `PaintOptions`, and the options
  JSON; map `"multiply"`; keep wet-edge strokes off the GPU path; apply the
  curve in the large-dab preview.
- [x] 2.2 Retune `Paintbrush Oval 45 pixels Multiply` from the screenshot
  measurements; every other preset clears Wet Edges.

## 3. Tests

- [x] 3.1 Engine tests for the curve, the wet-edge stroke tones, and Multiply.
- [x] 3.2 Qt tests for the oval preset's tip and Wet Edges persistence.

## 4. Verification

- [x] 4.1 Build, run the Qt suites and `bash scripts/verify-fast.sh`.
- [x] 4.2 Update `docs/dev/STATE.md` and archive the change.
