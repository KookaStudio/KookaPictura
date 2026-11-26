# Design

## Context

The generic adjustment page serves the destructive dialog and Properties. The dialog's existing Levels and Curves histogram reads the displayed composite. Threshold needs the samples before its own preview or layer operation.

## Decisions

- Reuse the themed HistogramView and insert it in the Threshold level control between the value row and slider. Read the dialog bins before launching its first preview, then keep that snapshot for its lifetime.
- For a destructive edit, read the same working RGB or gray planes as the adjustment, or editable grayscale when editing a raster mask or Quick Mask. For Quick Mask use `working_value` in the current polarity. Respect the selection by weighting samples by its coverage. Skip fully transparent pixels.
- For Properties, use a scratch copy of the layer stack ending just below the Threshold layer, with the target excluded. Preserve contributing lower siblings and pass-through context, and stop at an isolated group boundary. Preserve applicable selection coverage. Refresh when the lower source or current selection changes outside the slider gesture, including a coalesced refresh using translated coverage during a live selection move and after cancellation.
- Keep memory and sample work bounded. Count all samples from small sources and sample deterministic bounded tiles across large sources. Restrict a zero-exterior selection to its cached mask bounds so a small selection in a large document still has full coverage. Use the established Rec.601 luminance conversion for bin indices.

## Risks / Trade-offs

- Sampling can miss rare tones in large inputs or sparse selections. The histogram is a visual guide and small inputs retain their full distribution.
- The adjustment-layer chart represents the lower stack. Exact Photoshop clipping and knockout behavior is outside this lower-stack guide; more specialized clipping, knockout, and mask interactions continue to be handled by the compositor.
