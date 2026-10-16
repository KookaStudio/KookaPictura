# Design

## Preview on the canvas, not on a private image

The stack rides the existing single-filter preview core
(`apply_op_active_region`) as a new `ActiveOp::Filters`. It snapshots the layer
on the first preview, re-filters the snapshot on every change, and restores it
bit-identically on Cancel. Preview and commit therefore see the same layer,
selection mask, and lock rules, and the gallery's pane just shows the canvas
image at its own zoom. A private image path would have duplicated all of that
and could disagree with the commit on a multi-layer document.

## Section preview of a stack

Only the pane's visible document rect is filtered. Each filter in the stack
runs over the visible rect expanded by the support (`preview_apron`) of itself
and every filter after it, so the visible area is exact after the whole stack.
Positional kinds (`filter_preview_needs_whole_layer`) force a whole-layer
preview, as for a single filter.

## Thumbnails

CS6 ships fixed sample thumbnails; Kooka has no such assets. Each thumbnail is
the filter at its defaults over an 80×56 sample of the picture, rendered one
per event-loop turn so the dialog opens immediately. The sample is scaled and
packed once when the dialog opens, and every filter renders from it. Scale-dependent filters
look coarser on the sample than on the picture (ponytail: no fixed sample
image).

## Stack order

`effects_` is first-applied first; the list shows it reversed (top row = last
applied), matching CS6's effect layers. New effect layer inserts a copy of the
selected effect above it.
