# Proposal: add-vanishing-point-retouch

## Why

Issue #349, part 2. Part 1 (#919) shipped Vanishing Point's planes and
Render Grids. The tools that make the filter useful are still missing:
pasting and moving pixels in perspective, cloning, painting and picking
colour (`docs/06-filters/vanishing-point.md`, `FILT-102`).

## What Changes

- `pictura-filters` `distort::vanishing_point` gains:
  - **A flat editing space per plane family.** It is the root's metric,
    with related planes unfolded about their hinges. Edits are placed and
    sized there and mapped to the image through the plane under each pixel.
  - **A working copy of the layer**, with pixel undo steps in the dialog's
    undo.
  - **Floating selections.** Paste, the Marquee (rect, square, whole plane,
    Alt-drag copy, Destination / Source move, Ctrl-drag fill) and Transform
    (scale, rotate, move, Flip, Flop). Feather, Opacity and Heal apply when
    a selection is dropped.
  - **Stamp and Brush.** Round dabs sized in the flat space, with Diameter,
    Hardness, Opacity, Heal and the Stamp's Aligned. Plus an Eyedropper.
  - **Heal.** Off copies, Luminance re-lights, and On is a Poisson blend:
    the source's texture with the destination's colour and lighting.
  - **The edit carries a pixel patch.** `VpEdit` now holds the changed
    rect of the layer as well as the grid lines, so OK, Fade and Last Filter
    replay it through the ordinary filter path.
- `pictura-app`:
  - The bridge gains the tools, the clipboard paste, the preview region
    renderer and the commit patch.
  - The dialog gains the Marquee, Stamp, Brush, Transform and Eyedropper
    tools with per-tool options, Ctrl+V, Ctrl+T, Ctrl+D and `[` / `]`.
- Deferred: saving planes in files (#918). The Extended-only Measure tool
  stays out of scope.

## Capabilities

### Modified Capabilities

- `imaging/vanishing-point`: the commit applies pixel edits, the workspace
  gains the five tools and their options, and new requirements cover the
  editing space, floating selections, and Stamp, Brush and Eyedropper.

## Impact

- `pictura-filters`: `distort/vanishing_point/*`.
- `pictura-app`:
  - `filter_tools/vanishing_point*.rs`
  - `vp_dialog.*`, `vp_canvas.*` and new `vp_options.*`
  - `CMakeLists.txt`
- Tests: `tst_vanishing_point` (updated) and the new
  `tst_vanishing_point_retouch`.
