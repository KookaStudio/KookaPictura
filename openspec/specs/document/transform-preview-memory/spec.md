# document/transform-preview-memory Specification

## Purpose
The memory a transform session's preview holds: a large plain layer previews
at a reduced level, and opening a session asks the memory governor first, so a
large layer is refused with a message instead of exhausting memory.

## Requirements

### Requirement: A large plain layer previews at a reduced level

When a transform session opens on a single plain pixel layer (its channels are
its whole appearance, it heads no clipping group, it is not a smart object, and
no selection is lifted) whose box exceeds the preview threshold (16 Mpx), the preview's
moving image and backdrop patch SHALL be built at the smallest power-of-two
reduction that brings the box to at most the threshold, and the canvas SHALL
draw them scaled to their document size. A target at or under the threshold, or
one that is not plain, SHALL preview at full resolution.

#### Scenario: A large layer's preview is reduced [tpm_reduced]

- **WHEN** a Free Transform session opens on a plain pixel layer whose box is over the threshold and at most four times it
- **THEN** the preview scale is 2, and the moving image is half the box's size at the box's origin

#### Scenario: A small layer keeps full resolution [tpm_small_exact]

- **WHEN** a Free Transform session opens on a plain pixel layer at the threshold, or on a styled layer over it
- **THEN** the preview scale is 1 and the moving image has the layer's own size

#### Scenario: The reduced preview is drawn at document size [tpm_draw_scaled]

- **WHEN** the canvas draws a transform preview whose moving image and base patch are half size at scale 2, unrotated and unscaled
- **THEN** the drawn layer covers the same document pixels as its full-size layer would

#### Scenario: The CPU warp samples a reduced image [tpm_warp_scaled]

- **WHEN** a projective preview is warped from a half-size image with a source scale of 2
- **THEN** it covers the same output pixels as the warp of the full-size image

#### Scenario: A Warp preview keeps the session's level [tpm_warp_level]

- **WHEN** a Warp preset is applied in a session whose preview scale is 2
- **THEN** the warped moving image is half the size of the warped layer's box

### Requirement: Opening a transform session is gated on its preview

Before a transform session builds its preview, the application SHALL ask the
memory governor for the preview's bytes. When they do not fit, no session SHALL
open, the document SHALL stay as it was, and the transform memory message SHALL
be shown.

#### Scenario: Preview bytes follow the level [tpm_bytes]

- **WHEN** the preview bytes of a plain layer over the threshold, of one at it, and of a lifted selection are computed
- **THEN** each is 20 bytes a preview pixel at its level, and the lifted selection adds 12 bytes a canvas pixel

#### Scenario: A preview beyond the room is refused [tpm_gate]

- **WHEN** the governor's room is pinned below the preview bytes and Free Transform is chosen
- **THEN** no session opens, the layer is unchanged, and a memory refusal is recorded
