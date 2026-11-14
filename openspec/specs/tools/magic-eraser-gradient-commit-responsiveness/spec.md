# tools/magic-eraser-gradient-commit-responsiveness Specification

## Purpose
Keeps the Magic Eraser and the Gradient fast on large documents: a click or a
drag commits only the region it changed, a large Magic Eraser click runs as a
cancellable job, uniform coverage erases and alpha planes stay allocation-free,
and an opaque full-strength Normal fill writes the source directly.

## Requirements

### Requirement: The Magic Eraser commits only its changed region

A Magic Eraser click SHALL record one history state whose damage is the
document rectangle `magic_erase` changed, and SHALL refresh only that region.
It SHALL recomposite the whole document only when a layer style could spread the
change. A click that changes nothing SHALL record no state.

#### Scenario: A partial erase composites its region [mgc_magic_region]

- **WHEN** the Magic Eraser clicks one pixel of a textured document with a
  zero tolerance and no layer style
- **THEN** the frame is not rebuilt, the composite equals a full recomposite,
  exactly one "Magic Eraser" state is recorded, and undo restores the pixels

#### Scenario: A styled document recomposites whole [mgc_magic_styled]

- **WHEN** a document with a layer style is erased by the Magic Eraser
- **THEN** the document is recomposited whole and the composite equals a full
  recomposite

### Requirement: A large Magic Eraser click is a cancellable job

On an armed view and a document at or above the document-job threshold, the
Magic Eraser SHALL run its flood, anti-alias and erase on a worker as a
document job. Cancelling the job SHALL leave the document, its history and the
Fade record unchanged. A finished job SHALL land one "Magic Eraser" state and
SHALL record Edit ▸ Fade only then.

#### Scenario: Cancelling a Magic Eraser job changes nothing [mgc_magic_job]

- **WHEN** an armed large document is erased and the job is cancelled before it
  lands
- **THEN** the document pixels, the history index and the Fade availability are
  unchanged

### Requirement: A uniform coverage erase writes uniform tiles

When a target tile and the mask over it are uniform and the tile lies fully on
the canvas, `magic_erase` SHALL write the tile's new value as a `Uniform` tile
without visiting its samples. The result MUST equal the per-pixel erase, and a
tile whose value would not change SHALL report no damage.

#### Scenario: A whole-canvas erase writes uniform alpha [mgc_uniform_tiles]

- **WHEN** a uniform layer with a uniform opaque alpha plane is erased through a
  whole-canvas mask at full opacity
- **THEN** every alpha tile is `Uniform(0)`, the changed rectangle is the canvas,
  and the pixels equal the per-pixel erase

#### Scenario: A partial uniform erase keeps its exact result [mgc_uniform_parity]

- **WHEN** the same erase runs at 50 % opacity and through an anti-aliased mask
- **THEN** every channel equals the per-pixel reference

### Requirement: A Background's alpha plane is uniform

The alpha channel added when a Background becomes a layer, and the one
`ensure_alpha` adds to a layer without transparency, SHALL be a tiled uniform
plane with no sample allocation. Its bytes SHALL equal an opaque plane.

#### Scenario: Unlocking a Background allocates no alpha pixels [mgc_uniform_alpha]

- **WHEN** a Background is turned into a layer, and a layer without transparency
  is given an alpha channel
- **THEN** every alpha tile is `Uniform(255)` and the plane holds no dense tile

### Requirement: An opaque full-strength Normal fill writes the source

The shared fill pass SHALL write the source colour directly when the mode is
Normal, the effective strength is at least 1 and the source alpha is 255, and
SHALL keep the destination's alpha for `Alpha::Keep`. The result MUST equal the
generic blend, and the Gradient SHALL use this path for its default
full-opacity opaque ramp.

#### Scenario: The direct path equals the blend [mgc_opaque_fill]

- **WHEN** an opaque source is laid at full strength in Normal mode with
  `Alpha::Replace`, `Alpha::Keep` and `Alpha::Blend`
- **THEN** every result equals the generic Normal blend's, and a Gradient drawn
  at full opacity equals the reference gradient
