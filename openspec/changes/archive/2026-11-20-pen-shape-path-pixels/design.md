# Design

## Mode storage

`PenOptions` gains `int mode` (0 Shape, 1 Path, 2 Pixels), clamped 0–2 and
persisted with the other Pen fields. Shape is the default, matching CS6 and the
shape tools. The Pen and Freeform Pen pages both show the combo
(`optionsPenMode`); a `penOptionsChanged` signal keeps the two pages in step.

## When a path lands

The handler already knows when a drawing session ends: `path_close`, the Enter/
Esc `finishDrawing`, the Ctrl-click branch of `pressPen`, a Freeform release,
and `onDeactivate` while a subpath is being drawn. Shape and Pixels land on all
of them through a small `finishPath`/`landDrawnPath` pair. Path saves on close,
Enter/Esc, Ctrl-click, and a Freeform release; a tool switch only calls
`path_finish`, so switching to an anchor tool keeps editing the Work Path and
undo steps back the last anchor.

Anchor-tool gestures (Add, Delete, Convert) do not land: they edit the current
path, and their `onDeactivate` only calls `path_finish` as before.

## Shape

`shape_add_work_path_layer` reads the Work Path's components, inserts a solid
fill layer above the active layer with a vector mask for the first component,
adds the remaining components to the mask, clears the Work Path, selects the new
layer, recomposites its reach, and records one `"Pen Tool"` (or
`"Freeform Pen"`) state. The fill is the foreground colour; the shape is a
regular path, not a live shape, because a Pen outline has no parametric origin.

## Path

`path_close_and_save`, `path_finish_and_save`, and
`path_add_freeform_and_save` save a copy of the Work Path as the next
`Path <n>` saved path. They leave the Work Path in place so the Pen can resume
or keep editing it, and they keep the existing history label (`Close Path`,
`Save Path`, `Freeform Pen`) as one state. A saved path is a snapshot of the
path at that moment; later Work Path edits do not rewrite it.

The copy is only taken while the Work Path is the active path. Editing a saved
path or a vector mask with the Pen does not create another saved path.

## Pixels

Pixels mode strokes the path with `paths_stroke` using the current Brush size,
hardness, opacity, flow, and spacing and the foreground colour. The stroke
records `"Stroke Path"`. The Work Path is left in place.

## Alternatives

- Moving the Work Path into a saved path (and clearing it) matches
  `paths_save_work` but breaks drawing-then-editing-anchors: the next Pen press
  would extend the saved path, not a fresh Work Path. A copy keeps both.
- Building a `ShapeSpec` for the Pen outline is impossible: `ShapeSpec` carries
  parametric geometry, while the Pen already has arbitrary Bézier components.
