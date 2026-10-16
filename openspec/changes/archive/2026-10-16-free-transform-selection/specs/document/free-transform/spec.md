## ADDED Requirements

### Requirement: Free Transform of selected pixels

When a non-empty, document-sized selection is active and the target is a pixel layer or the Background that is not pixel-locked, not transparency-locked while it has alpha, and not position-locked unless it is the Background, beginning a Free Transform session (in any mode) SHALL lift the selected pixels: they are copied into a floating layer directly above the target, trimmed to the selection bounds, and cleared from the target as `Edit > Clear` clears them. The session SHALL then transform the floating layer. Committing a non-identity transform SHALL composite the floating layer's pixels back over the target's own channels and remove it, leaving every other target attribute (name, locks, Background status, opacity, blend, mask, effects) unchanged; a target with alpha SHALL grow to hold pixels moved past its rect, and the Background SHALL clip them to its rect. The floating layer SHALL be sized to the selection bounds, not the document. A selection whose pixels are already the clear colour (white on a Background) SHALL still lift. A commit SHALL record exactly one `"Free Transform"` history state and SHALL drop the selection. Cancelling, or committing an identity transform, SHALL restore the document exactly as it was before the lift and record nothing. `layer_can_free_transform` SHALL report such a target as transformable, so `Edit > Free Transform` is enabled for it. Without a selection, the existing refusals (including the Background) SHALL apply unchanged.

#### Scenario: A Background with no selection stays refused

- **WHEN** Ctrl+T is pressed on an opened photo whose only layer is the Background and no selection is active
- **THEN** `Edit > Free Transform` is disabled and no session begins

#### Scenario: A selection on the Background is lifted and transformed

- **WHEN** a rectangle is selected on the Background, Ctrl+T is pressed, the box is dragged 10 px right and 5 px down, and Return commits
- **THEN** one `"Free Transform"` state is recorded, the document still has a single Background layer, the vacated pixels are white, the moved pixels land at the offset, and the selection is gone

#### Scenario: Lifting and merging back leaves the layer exactly as it was

- **WHEN** a selection on a layer with 50 % opacity, Multiply blend, and a mask is lifted and put back untransformed
- **THEN** the document equals the one before the lift

#### Scenario: Cancel restores the lifted pixels

- **WHEN** a session that lifted a selection is cancelled with Escape
- **THEN** the floating layer is gone, every pixel is as before, and no history state is recorded

### Requirement: Free Transform's box hugs the layer content

Beginning a Free Transform session on a whole pixel layer (no lifted selection) SHALL first trim the layer's rect to the bounds of its non-transparent pixels when it has alpha and no mask, so the transform box and its handles surround the visible content rather than the canvas-sized rect. Only fully transparent pixels SHALL be dropped, so the composite is unchanged and no history state is recorded for the trim.

#### Scenario: Ctrl+T on a canvas-sized layer boxes its content

- **WHEN** Ctrl+T is pressed on a canvas-sized layer whose only pixels are a 12 × 9 square at (10, 8)
- **THEN** the transform quad is that square's bounds, and cancelling leaves the composite unchanged

## MODIFIED Requirements

### Requirement: Skew, Distort, and Perspective menu commands

The application SHALL provide implemented commands `Edit > Transform > Skew`,
`Edit > Transform > Distort`, and `Edit > Transform > Perspective` that begin a
Free Transform session in the matching mode on the active layer, using the same
active-layer resolution and refusal as `Edit > Free Transform`. Each command
SHALL be enabled exactly when `Edit > Free Transform` is enabled, including for
a target whose selected pixels Free Transform lifts.

#### Scenario: The Distort command enters a Distort session

- **WHEN** `Edit > Transform > Distort` is invoked with exactly one transformable layer active
- **THEN** a session begins in `Distort` mode on that layer

#### Scenario: A non-transformable target leaves the command disabled

- **WHEN** the active layer is a group, an adjustment layer, or position-locked, or is the Background with no active selection
- **THEN** the Distort/Skew/Perspective commands are disabled and invoking them begins no session
