# Design

## Context

The stacked slider dialog already provides Auto dispatch and scratch-block
preview. The shared Properties actions and both preset menus already use one
per-kind store. See [Adobe's CS6 guide](https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf)
for the manual Vibrance controls and the seven documented preset kinds.

## Decisions

- Compute the recommendation in the existing Rust Image Adjustments bridge
  from the hand-laid `AdjustmentDialog` constructor, only when the decoded
  block is Vibrance, immediately before its opening `preview()` call. Sample
  the opening RGB raster's
  editable color planes with an evenly spaced grid of at most 128 samples
  per axis. Intersect its bounds with the selection when present. The raster
  sample avoids colors from other layers and display-cache timing.
- Ignore samples with zero alpha, zero selection coverage, or a channel spread
  below four 8-bit levels. Weight the remaining HSL saturation by alpha and
  selection coverage. Fit a positive Vibrance value toward mean saturation
  0.50, using the existing operator's nominal `1 - S` gain and capping the
  slider recommendation at +35. Zero refers to a no-op recommendation.
- Keep the opening recommendation for the dialog lifetime. Auto only writes
  Vibrance, replacing even a higher current value; it leaves Saturation
  unchanged, including at -100. The existing scratch block, preview timer, Preset matching,
  Cancel and OK manage the resulting edit. A mask or non-RGB target has no
  color recommendation.
- Add Vibrance to the shared eligibility list. Save and Load operate on the
  existing `vibA` block through the store used by the other adjustment kinds.
  The existing Default and Custom entries cover neutral and unnamed settings.

## Risks and trade-offs

Fixed-grid sampling can miss small islands in large sparse rasters. The
selection's bounding box focuses samples on small selections. Higher-depth
documents use their 8-bit rendering mirror for the recommendation. The fit is
a Pictura approximation. Adobe preset-file interchange and Properties Auto
are outside the direct-dialog scope of #564.

## Migration plan

Existing user preset files and PSD adjustment payloads retain their formats.
