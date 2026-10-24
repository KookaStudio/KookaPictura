# Design

## One model, two pictures

The crop session keeps one model: the content turned by `angle` about `pivot`
behind an axis-aligned box. Turning the content by +a behind a box is the same
crop as turning the box by -a over the content, so commit is unchanged
(`straighten_crop`). Classic draws the second picture: the composite stays
upright and the box overlay (shield cut-out, guides, handles, outline) is
drawn under a rotation of -angle about the pivot (`ImageView::setCropBoxRotates`).

Pointer input arrives in the picture's frame. `CropToolHandler::modelPos` maps
it back by +angle about the pivot, so grips, moves, hover cursors and the new
box all work in the model unchanged. The rotate gesture reads the pointer in
the picture's frame around the box's on-screen centre; the model angle moves
opposite to the sweep. A straighten line is drawn on the fixed image, so
Classic levels the box against it without adding to a prior angle.

Before a gesture sets a new angle (which re-seats the pivot at the box
centre), `settlePivot` moves an already turned box to its on-screen centre so
it does not jump.

ponytail: snapping is off while a Classic box is turned, since the canvas edges
are not axis-aligned with it; the canvas frame does not grow around a turned
Classic box.
