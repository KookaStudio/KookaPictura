# Design: rulers-and-guides

## Where guides live

Guides are document data (`Document.guides`, a `Vec<Guide>` of orientation +
position in document pixels). History snapshots clone a document's metadata,
so guide edits undo and redo with no history change, and the PSD carries them.
Rulers, Show Guides, and Lock Guides are view state owned by the frame and
persisted in the session store, as CS6 keeps them across launches.

## Rulers

`CanvasScrollBars` (the per-document canvas host) gains a 3×3 grid: a corner
and the horizontal ruler on top, the vertical ruler on the left, the canvas,
and the scrollbars. A `CanvasRuler` paints in the canvas's unrotated view
frame (`widget = image × zoom + offset`), so its 0 sits on the document's
left/top edge. A unit converts through the document's ppi (`document_ppi`,
read at paint time so Image Size changes show at once); percent uses the
document's width or height. `CanvasRuler::scaleFor` picks the smallest major
step from 1, 2, 5 × 10ⁿ units (whole pixels for the pixel unit) that is at
least 50 screen pixels wide, divided into eighths for a 1-inch step (CS6's
ruler) and otherwise 10 / 4 / 5 minor ticks, halved while closer than 4
screen pixels and never splitting a pixel. Labels are the unsigned distance
from 0, as CS6 draws them.

A press on a ruler grabs the mouse; each move maps the global cursor into the
canvas and shows a preview guide; a release inside the canvas rect emits
`guideDropped(vertical, position)`, which the frame turns into `add_guide`.
Positions round to whole pixels.

## Guide drags

`ToolController::handlePressed` asks `beginGuideDrag` first: with guides shown
and unlocked, and the Move tool active or `Ctrl` held, a press within 4 screen
pixels of a guide (`guide_near` with a radius of `4 / zoom`) starts a drag.
Moves are live (`move_guide` with `commit = false`: no history, no
`changed`); the release commits "Move Guide", or "Delete Guide" when the
cursor is outside the canvas widget. The Move tool shows a split cursor over
a guide.

## PSD 1032

The read decodes 1032 into `guides`. The write re-emits the section untouched
when it already decodes to the document's guides; otherwise it replaces 1032
(keeping its grid cycles, else 576 / 576) or drops it when there are none.
Location is `round(position × 32)` as a signed 32-bit integer.

## Preferences

The page stores one colour (`#rrggbb`) and a dashed flag; the colour combo
shows the preset whose colour matches, else Custom…. Custom… opens a
`QColorDialog`. Cyan `#4AFFFF` and Magenta `#FF4AFF` were sampled from the CS6
screenshot on #294; the other presets are approximations.
