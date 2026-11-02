# Design

## Where the time went

No metric covers a transform press, so the GUI thread was sampled. A press made
two whole-document composites and two whole-canvas conversions, one pair of each to
render the layer and one for the backdrop, then copied the 1 GB backdrop into Qt.

## Backdrop patch

During a transform the document does not change, so the canvas is right everywhere
except under the target. The backdrop only has to replace the target's bounds. The
commit's region refresh relies on the same bound, so the rule matches it: no patch
for effects, an adjustment, or a group. A lifted selection also keeps the whole
backdrop, because the canvas still shows the document from before the lift.

The canvas draws its normal level, then the patch. A patch can be transparent, so
its area is first cleared to the checkerboard, or the target would show through.

The Move drag uses the same patch. It used to replace the canvas with an unsmoothed
full-size backdrop, so a click on a layer visibly changed the image at any zoom
under 100 %. A selection drag and a layer that keeps the whole backdrop still draw
that image, now smoothed; far below 50 % it stays coarser than the canvas.

## The stall after release

A region edit forks a band of the composite. The next `sample_planar_argb` indexed
the plane, which materializes the whole composite to read four bytes. `Plane::sample`
reads the band directly. This was hidden behind the press in the first measurements.

## Measurements

A text layer about 190 px wide on `worldmap_16000x16000.png` (16507×16196) with three
layers and GPU compute on, in milliseconds. Headless, machine load about 10:

| Phase | Before | After |
|---|---|---|
| First press | 13455 | 142 |
| Later presses | 4500–4700 | 2–6 |
| Move, median | 1–3 | about 1 |
| Release reply | 203–236 | 4–8 |
| Release until idle, worst run | 3603 | 150 |

## Limits

- A full-canvas layer still composites the whole document, as a patch of the whole
  document, and a clipping group's moving image still renders in full.
- ponytail: the patch is drawn unrotated, as the whole-document base always was, so
  a rotated view shows it out of line with the canvas under it.
