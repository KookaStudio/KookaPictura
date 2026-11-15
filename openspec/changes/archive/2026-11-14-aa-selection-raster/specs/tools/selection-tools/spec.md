# Spec Delta

## MODIFIED Requirements

### Requirement: Magic Wand

`magic_wand(image, x, y, tolerance, contiguous, anti_alias)` SHALL build a
selection from the colour at the seed point. A pixel MUST be selected when its
per-channel maximum absolute difference (Chebyshev distance) from the reference
colour is less than or equal to `tolerance`. When `contiguous` is true, only
pixels four-connected to the seed MUST be selected; when false, every matching
pixel in the image MUST be selected. A seed outside the image, or an empty
image, MUST return an error. The enabled Magic Wand tool SHALL route a canvas
click through this function using the options-bar Tolerance (0-255),
Contiguous, and Anti-alias state, and SHALL combine the result with the current
selection using the selected New, Add, Subtract, or Intersect mode.

When `anti_alias` is true and `tolerance` is greater than zero, after the
matched set is built, each unselected pixel four-adjacent to a selected pixel
whose Chebyshev distance `d` from the reference colour satisfies
`tolerance < d < tolerance + width` SHALL receive partial coverage
`round(clamp(1 - (d - tolerance) / width, 0, 1) * 255)`, where
`width = max(0.5 * tolerance, 1.5)`. Selected pixels and pixels at
`d >= tolerance + width` SHALL keep their binary coverage. When `anti_alias` is
false or `tolerance` is zero, every coverage value SHALL be 0 or 255. The
window is inferred from the GIMP fuzzy-select model; Adobe's exact anti-alias
ramp is not published. The `max(..., 1.5)` floor keeps Tolerance 1 and 2 able
to produce a fringe.

The tool SHALL offer Anti-alias and Sample All Layers controls. Anti-alias
SHALL be enabled and functional. Sample All Layers SHALL be visible and
disabled with a reason because the wand samples the visible composite.
Contiguous SHALL default to on and Tolerance to 32.

#### Scenario: Contiguous flood stops at a colour boundary

- **WHEN** the wand is seeded in a red region touching a blue region with a tolerance that excludes blue
- **THEN** only the connected red pixels are selected

#### Scenario: Global mode selects disconnected matches

- **WHEN** the wand runs non-contiguous on an image with two disconnected identical-coloured patches
- **THEN** pixels in both patches are selected

#### Scenario: Tolerance rejects distant colours

- **WHEN** Anti-alias is off and a neighbouring pixel differs from the reference colour by more than `tolerance`
- **THEN** that pixel is not selected

#### Scenario: Out-of-bounds seed errors

- **WHEN** the seed coordinates fall outside the image
- **THEN** the wand returns an error instead of panicking

#### Scenario: The tool combines with the current selection

- **WHEN** a selection exists, Add mode is active, and the Magic Wand clicks a region
- **THEN** the committed selection is the union of the existing selection and the wand result in one undo state

#### Scenario: Anti-alias grades the wand boundary [wand_aa_fringe]

- **WHEN** Anti-alias is on with a positive Tolerance and a pixel next to the matched set lies between `tolerance` and `tolerance + width` from the reference colour, where `width = max(0.5 * tolerance, 1.5)`
- **THEN** that pixel receives coverage strictly between 0 and 255, while pixels inside the tolerance stay 255 and pixels at or beyond `tolerance + width` stay 0

#### Scenario: Anti-alias grades at a low tolerance [wand_aa_low_tolerance]

- **WHEN** Anti-alias is on with Tolerance 1 and a neighbouring pixel is at Chebyshev distance 2 from the reference colour
- **THEN** that pixel receives coverage strictly between 0 and 255, while pixels at distance 0 stay 255 and pixels at distance 3 or more stay 0

#### Scenario: Anti-alias off or zero tolerance stays binary [wand_aa_off]

- **WHEN** the wand runs with Anti-alias off, or with Tolerance 0 and Anti-alias on
- **THEN** every coverage value is 0 or 255

#### Scenario: Anti-alias surrounds each global match [wand_aa_global]

- **WHEN** the wand runs non-contiguous with Anti-alias on
- **THEN** each matched region receives its own partial boundary fringe and no fringe appears between regions where no pixel matches

#### Scenario: Wand options are modelled honestly [wand_aa_options]

- **WHEN** the Magic Wand options bar is shown
- **THEN** Anti-alias is enabled, while Sample All Layers is disabled and states why it has no effect

#### Scenario: Unmodelled wand options are honest

- **WHEN** the Magic Wand options bar is shown
- **THEN** Anti-alias is enabled and functional, while Sample All Layers is disabled and states why it has no effect
