# compositing/layer-masks Specification (delta)

## ADDED Requirements

### Requirement: Mask editing is grayscale-native and accelerated

The system SHALL edit a raster layer mask's single coverage plane. The
document through which a mask is painted SHALL carry the coverage as one
grayscale channel, so a dab writes one byte per pixel and no per-dab channel
copy-back is required for correctness. A mask stroke SHALL honour the same
interactive-acceleration policy as a pixel-layer stroke: a dab within the raster
budget SHALL use the exact CPU raster, and a larger brush SHALL use a
reduced-level preview and/or a GPU stroke, selected by the same budget rule.
The committed coverage SHALL be byte-identical, within the compositor's
one-unit-per-channel tolerance, to the exact CPU raster of the same sample log.
A mask stroke SHALL present the changed region only; it SHALL NOT rebuild the
whole mask document, the whole pyramid, or recomposite the whole document on a
per-dab basis. Fills and filters on the mask SHALL keep using the same
single-channel document.

#### Scenario: A dab writes the coverage plane directly

- **WHEN** a mask stroke paints a dab
- **THEN** only the mask's coverage byte changes and no RGBA mask-sized
  intermediate is allocated or copied back

#### Scenario: A large mask brush uses the accelerated path

- **WHEN** the mask target is active and the brush diameter exceeds the raster
  budget
- **THEN** the stroke uses the reduced-level preview and/or the GPU path in place
  of the exact full-resolution CPU raster per dab

#### Scenario: The committed mask matches the exact raster

- **WHEN** a mask stroke is committed after a previewed or GPU stroke
- **THEN** the committed coverage matches a full-resolution exact CPU raster of
  the same sample log within one unit per channel

#### Scenario: The per-dab present stays region-bounded

- **WHEN** a mask dab changes coverage
- **THEN** only the dab's region is recomposited and presented, and the whole
  document is not recomposited

#### Scenario: A live mask stroke renders while dragging

- **WHEN** a mask stroke is dragged across several dabs in one frame
- **THEN** the canvas presents the accumulated region on that frame (a live mask
  stroke counts as painting, so the shell's deferred present runs) instead of
  showing only the first dab and the rest on release
