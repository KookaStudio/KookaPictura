# Design

Use document PPI for both axes, and the ruler's existing points-per-inch
definition for Picas. Twelve points make one pica. Read the saved column
width and gutter in inches when the dialog opens. Older files get 2.5 inches
and 1/6 inch, the current Units & Rulers placeholders of 180 and 12 PostScript
points. Use the same preference keys as #565.

An absolute span of one column is the width. Greater spans add a gutter
between adjacent columns, so two columns measure twice the width plus one
gutter. Fractional spans below one scale the width. Relative mode measures
signed pitch (width plus gutter) so adding one column to a one-column span
yields a two-column span and zero leaves the canvas alone. The absolute
formula and its inverse are continuous at one column. Show enough precision
for a restatement in Picas or Columns without losing pixel dimensions. Feed
resolved pixel dimensions to the existing resize command and action recorder.

The local Canvas Size spec calls for units on both axes. Adobe describes
column sizing as a width operation and documents width and gutter preferences,
but does not specify fractional or relative Canvas Size arithmetic. The
two-axis UI and those conversions follow the choices above.
[Adobe column sizing](https://helpx.adobe.com/in/photoshop/using/specifying-columns-image.html)

Check for a top-level, non-group Background before resize, both for the modal
controls and for the fill decision in the resize transaction. Disable both
entry points to the picker when there is none. Keep the existing Background
fill path when one was present. Resize may materialize a layerless document's
matching merged composite as a Background to preserve its pixels; that newly
created layer MUST NOT authorize extension fill for this operation. Skip the
fill in that case so uncovered added canvas is transparent, while retaining the
materialized layer and its original pixels as required by `ui/document-canvas`.
The decision belongs to the transaction as well as the dialog, since Actions
and other callers can invoke the resize without opening the dialog.

Resize translates existing layer bounds rather than clipping them. Growth may
therefore reveal retained content beyond the old canvas; only areas not covered
by such content are transparent when the original document has no Background.
[Adobe Canvas Size](https://helpx.adobe.com/ca/photoshop/using/adjusting-crop-rotation-canvas.html)
