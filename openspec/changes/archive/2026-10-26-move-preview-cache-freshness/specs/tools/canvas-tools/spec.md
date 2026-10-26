## ADDED Requirements

### Requirement: Move preview freshness

The Move tool's live preview SHALL place the moving layers at the pointer's
offset. A prepared preview SHALL be reused only while the moving layers have not
moved: a rendered link set's preview image is composited at the layers'
positions and clipped to the canvas, so a move makes the cached image stale and
the next drag SHALL recompute it rather than draw the stale image at the new
origin.

#### Scenario: A moved link set recomputes its preview [lct_move_preview_fresh]

- **WHEN** a linked set is moved and the Move tool begins another drag without
  moving it again
- **THEN** the moved set's preview is recomputed at the new union rect and the
  set previews and lands at the pointer's offset
