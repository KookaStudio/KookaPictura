# selection-tools Specification

## Purpose
Magic Wand, Grow, Similar, Color Range, and saving or loading a selection to an alpha channel.
## Requirements
### Requirement: Magic Wand

`magic_wand(image, x, y, tolerance, contiguous)` SHALL build a selection from
the colour at the seed point. A pixel MUST be selected when its per-channel
maximum absolute difference (Chebyshev distance) from the reference colour is
less than or equal to `tolerance`. When `contiguous` is true, only pixels
four-connected to the seed MUST be selected; when false, every matching pixel in
the image MUST be selected. A seed outside the image, or an empty image, MUST
return an error. The enabled Magic Wand tool SHALL route a canvas click through
this function using the options-bar Tolerance (0-255) and Contiguous state, and
SHALL combine the result with the current selection using the selected New, Add,
Subtract, or Intersect mode. The tool SHALL offer Anti-alias and Sample All
Layers controls; because the engine rasteriser is binary and `current_buffer` is
the visible composite, both SHALL be visible and disabled with a reason.
Contiguous SHALL default to on and Tolerance to 32.

#### Scenario: Contiguous flood stops at a colour boundary

- **WHEN** the wand is seeded in a red region touching a blue region with a tolerance that excludes blue
- **THEN** only the connected red pixels are selected

#### Scenario: Global mode selects disconnected matches

- **WHEN** the wand runs non-contiguous on an image with two disconnected identical-coloured patches
- **THEN** pixels in both patches are selected

#### Scenario: Tolerance rejects distant colours

- **WHEN** a neighbouring pixel differs from the reference colour by more than `tolerance`
- **THEN** that pixel is not selected

#### Scenario: Out-of-bounds seed errors

- **WHEN** the seed coordinates fall outside the image
- **THEN** the wand returns an error instead of panicking

#### Scenario: The tool combines with the current selection

- **WHEN** a selection exists, Add mode is active, and the Magic Wand clicks a region
- **THEN** the committed selection is the union of the existing selection and the wand result in one undo state

#### Scenario: Unmodelled wand options are honest

- **WHEN** the Magic Wand options bar is shown
- **THEN** Anti-alias and Sample All Layers are disabled and state why they have no effect

### Requirement: Grow

`grow(selection, image, tolerance)` SHALL add pixels adjacent to the current
selection whose colour is within `tolerance` of the colour of the selected pixel
they touch, expanding the selection contiguously and never adding disconnected
regions. The selection and image dimensions MUST match, and a mismatch MUST
return an error.

#### Scenario: Grow expands within a patch

- **WHEN** a single selected pixel lies inside a uniform patch next to a differently coloured region
- **THEN** the whole patch becomes selected and the different-coloured region does not

#### Scenario: Grow does not jump globally

- **WHEN** two same-coloured patches are separated by a different colour
- **THEN** growing from one patch does not select the other

#### Scenario: Size mismatch errors

- **WHEN** the selection and image dimensions differ
- **THEN** `grow` returns an error

### Requirement: Similar

`similar(selection, image, tolerance)` SHALL add every pixel in the image whose
colour is within `tolerance` of the colour of any currently selected pixel,
regardless of adjacency. Selection and image dimensions MUST match, and a
mismatch MUST return an error.

#### Scenario: Similar reaches disconnected patches

- **WHEN** two same-coloured patches are separated by a different colour and one is selected
- **THEN** similar selects the other patch as well

#### Scenario: Similar rejects other colours

- **WHEN** a pixel's colour is farther than `tolerance` from every selected colour
- **THEN** it is not selected

#### Scenario: Size mismatch errors

- **WHEN** the selection and image dimensions differ
- **THEN** `similar` returns an error

### Requirement: Color Range

`color_range(image, target, fuzziness)` SHALL produce soft coverage from a
colour and a fuzziness value. For a pixel at Chebyshev distance `d` from
`target`, coverage MUST be 255 when `d` is 0, 0 when `d` is greater than or
equal to `fuzziness`, and otherwise `round((1 - d / fuzziness) * 255)`. A
fuzziness of 0 MUST select exact colour matches only. Increasing fuzziness MUST
never shrink the selected set.

#### Scenario: Exact match at zero fuzziness

- **WHEN** `fuzziness` is 0
- **THEN** only pixels equal to the target colour receive non-zero coverage

#### Scenario: Wider fuzziness never shrinks the set

- **WHEN** fuzziness increases across a series of values
- **THEN** the count of non-zero pixels is non-decreasing

#### Scenario: Soft ramp at intermediate distances

- **WHEN** a pixel lies closer to the target than the fuzziness value but does not match it exactly
- **THEN** its coverage is strictly between 0 and 255

### Requirement: Save and load to an alpha channel

The system SHALL convert a selection to an 8-bit grayscale alpha `Channel`
(`to_channel(id)`) by copying the coverage bytes, and SHALL reconstruct a
selection from a channel (`from_channel(channel, width, height)`) when the
channel length equals `width * height`. A save followed by a load MUST round-trip
byte for byte and MUST preserve the channel id. A channel whose length does not
match MUST return an error.

#### Scenario: Round trip is exact

- **WHEN** a selection is saved to a channel and loaded back at the same dimensions
- **THEN** the reloaded selection equals the original byte for byte and the channel id is preserved

#### Scenario: Wrong-length channel errors

- **WHEN** the channel byte count is not `width * height`
- **THEN** loading returns an error

### Requirement: Shared Magic Wand tolerance for Grow and Similar

The Magic Wand options-bar Tolerance SHALL be the single tolerance source for
`Select > Grow` and `Select > Similar`, so a Grow or Similar run after a wand
selection uses the value currently shown in the wand options bar. Grow and
Similar SHALL operate against the visible composite buffer and SHALL return an
error when the selection and buffer dimensions differ.

#### Scenario: Grow uses the shown tolerance

- **WHEN** the wand options bar shows Tolerance 40 and Select > Grow is run
- **THEN** growth accepts neighbours within 40 levels of the touched selected colour

#### Scenario: Dimension mismatch is refused

- **WHEN** the selection dimensions differ from the buffer and Select > Similar is run
- **THEN** the command is refused, the selection is unchanged, and no history state is recorded

### Requirement: Selection from a layer's alpha channel

The system SHALL provide a bridge operation that builds a document-sized
selection from a layer's alpha channel. For each document pixel, the selection
coverage SHALL be the layer's alpha value at that pixel's layer-local coordinate
(`document − layer.rect.top_left`); when the layer has no alpha channel the
coverage SHALL be `255`, and a pixel outside the layer's `rect` SHALL be `0`. The
resulting selection SHALL be combined with the current selection using the New
mode (it replaces the current selection) and SHALL be recorded as one undoable
state. An unresolved layer path SHALL be refused without changing the selection
or the document. The existing `pictura_select::Selection::from_channel`
construction SHALL be reused rather than reimplemented.

#### Scenario: A layer's alpha shape becomes the selection [lst_from_alpha]

- **WHEN** the bridge operation runs for a layer with an alpha channel
- **THEN** the resulting selection covers exactly the pixels where the layer's
  alpha is non-zero, offset to the layer's document position

#### Scenario: A missing alpha channel is fully opaque [lst_from_alpha_missing]

- **WHEN** the bridge operation runs for a layer with no alpha channel
- **THEN** every pixel inside the layer's `rect` has coverage `255` and every
  pixel outside it has coverage `0`

#### Scenario: The result replaces the current selection [lst_from_alpha_new]

- **WHEN** a selection already exists and the operation runs
- **THEN** the current selection is replaced by the layer's alpha shape as one
  undoable state

#### Scenario: An unresolved path is refused [lst_from_alpha_bad_path]

- **WHEN** the bridge operation is given a path that does not resolve to a layer
- **THEN** it returns a refusal and the selection and document are unchanged

### Requirement: Marquee Alt two-stage behavior

During a marquee drag, the first Alt press SHALL only subtract from the existing
selection. A second Alt press during the same drag SHALL switch to pivot-mirror,
so the rectangle mirrors about the anchor. The two behaviours SHALL NOT apply
from a single Alt press.

#### Scenario: First Alt subtracts [uistalt_first_subtract]

- **WHEN** an Alt press begins a marquee drag with an existing selection
- **THEN** the drag subtracts from the selection and does not mirror the rectangle

#### Scenario: Second Alt mirrors [uistalt_second_mirror]

- **WHEN** Alt is pressed a second time during the same drag
- **THEN** the rectangle mirrors about the anchor point

