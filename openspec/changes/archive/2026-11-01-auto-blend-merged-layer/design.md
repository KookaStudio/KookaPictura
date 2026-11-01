# Design

## Multiband blend

`blend.rs` works at full resolution over the union of content, one layer at a
time. Each layer's Laplacian pyramid is added to a running result pyramid, weighted
by a Gaussian pyramid of its label times its alpha, then the result collapses.
Transparent pixels take the layer's mean colour first, so edges do not darken.

Where Panorama layers disagree, each level's weight moves to the hard label, so
low bands of an object on one side of a seam do not ghost across it.

## Output

Source layers keep their pixels and gain masks. The merged layer goes above the
topmost source and any clipped run on it, so no source path moves. A layer directly
there named exactly `<top> (merged)` is replaced.

## Content Aware Fill

Runs `heal_region` over the transparent canvas pixels of the merged layer, which
then spans the canvas. `RgbaImage` is 8-bit, so the option is ignored at 16/32-bit
and the dialog disables it.

## Limits

32-bit samples above 1.0 clamp in the merged layer, as the compositor already
clamps them. Memory follows the blended area, unlike the bounded mask analysis.
