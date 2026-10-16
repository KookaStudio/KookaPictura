## ADDED Requirements

### Requirement: Refine Edge edge estimation

The system SHALL provide `pictura_select::refine::refine(mask, image, settings) -> Selection` that refines the coverage of `mask` against the composite `image`. The refinement MUST be confined to a boundary band: no pixel farther than the effective radius from the input 50 % contour SHALL change coverage. With `smart_radius` off the effective radius SHALL be a constant `radius`; with `smart_radius` on it SHALL narrow where the image edge is hard (a large local luminance gradient) and widen where it is soft, scaled by `radius`. Within the band the coverage SHALL be re-estimated from the image and blended back to the input so the band edge is continuous. The result SHALL be deterministic and SHALL be bit-identical to the input when every refinement is at its default (`radius` 0, `smooth` 0, `feather` 0, `contrast` 0, `shift_edge` 0). A `radius`, `smooth`, or `contrast` outside `0..=100`, an `amount` outside `0..=100`, or a `shift_edge` outside `-100..=100` SHALL be rejected with `SelectError::InvalidParams`. A mask and image of different dimensions SHALL be rejected with `SelectError::SizeMismatch`.

#### Scenario: Defaults are the identity

- **WHEN** `refine` runs with `radius` 0, `smooth` 0, `feather` 0, `contrast` 0, and `shift_edge` 0
- **THEN** the returned coverage is byte-identical to the input mask

#### Scenario: A larger radius reaches farther but stays bounded

- **WHEN** `refine` runs on a hard-edged square selection at a small radius and at a larger radius
- **THEN** the set of pixels whose coverage changed at the larger radius is a superset of the small radius' set, and no changed pixel lies farther than the larger radius from the square's edge

#### Scenario: Out-of-range parameters are rejected untouched

- **WHEN** `refine` is called with `radius` 101, `smooth` 101, `contrast` 101, `amount` 101, or `shift_edge` -101
- **THEN** it returns `SelectError::InvalidParams`

#### Scenario: A size mismatch is an error

- **WHEN** `refine` is called with a mask whose dimensions differ from the image
- **THEN** it returns `SelectError::SizeMismatch`

### Requirement: Refine Edge global refinements

After edge estimation the system SHALL apply Smooth (majority filter over the `smooth` radius), Feather (Gaussian blur of the `feather` radius), Contrast (steepen the coverage ramp about 128 by `contrast` percent so partial values move toward 0 or 255), and Shift Edge (move the 50 % contour inward for negative and outward for positive `shift_edge` percent). Contrast SHALL reduce the count of intermediate coverage values monotonically with `contrast`. Shift Edge SHALL move the 50 % contour monotonically with its value in the documented direction.

#### Scenario: Contrast makes the histogram more bimodal

- **WHEN** `refine` runs on a feathered selection with growing `contrast`
- **THEN** the number of pixels with intermediate coverage (0 < c < 255) decreases monotonically

#### Scenario: Shift Edge moves the contour the documented way

- **WHEN** `refine` runs with a negative and then a positive `shift_edge` on a soft selection
- **THEN** the 50 % boundary moves inward for the negative value and outward for the positive value

#### Scenario: Smooth removes boundary specks

- **WHEN** `refine` runs with `smooth` greater than 0 on a boundary carrying isolated single-pixel specks
- **THEN** the number of isolated specks decreases

### Requirement: Color decontamination

The system SHALL provide `pictura_select::refine::decontaminate(image, alpha, amount) -> PixelBuffer` that replaces colour fringes at soft edges with the colour of nearby fully selected pixels. For each pixel the foreground colour SHALL be estimated from fully-opaque selected neighbours and the output colour SHALL be the input colour lerped toward it by `(1 - alpha_normalised) * amount_percent`. The function SHALL return a new buffer and MUST NOT mutate its input. A pixel whose alpha is fully opaque (255) or fully zero SHALL be unchanged; a flat region with no usable foreground SHALL fall back to leaving the fringe rather than producing a non-finite or out-of-range sample. `amount` outside `0..=100` SHALL be rejected with `SelectError::InvalidParams`, and a size mismatch SHALL be rejected with `SelectError::SizeMismatch`.

#### Scenario: An edge pixel moves toward the nearby foreground

- **WHEN** `decontaminate` runs on an alpha edge between a red foreground and a green fringe with `amount` 100
- **THEN** the fringe pixel's red channel increases and its green channel decreases relative to the input

#### Scenario: Fully opaque and fully transparent pixels are untouched

- **WHEN** `decontaminate` runs with any `amount`
- **THEN** every pixel whose alpha is 0 or 255 has byte-identical colour

#### Scenario: Out-of-range amount is rejected

- **WHEN** `decontaminate` is called with `amount` -1 or 101
- **THEN** it returns `SelectError::InvalidParams`

### Requirement: The Refine Edge dialog and command

The app SHALL implement `Select ▸ Refine Edge…` (`Ctrl+Alt+R`) and a `Select and Mask…` button in the selection tools options bar, both opening the non-modal `RefineEdgeDialog`. The dialog SHALL expose View Mode (Marching Ants, Overlay, On Black, On White, Black & White, On Layers, Reveal Layer), Show Radius, Show Original, Smart Radius, Radius (0–100 px), Smooth (0–100), Feather (0–100 px), Contrast (0–100 %), Shift Edge (−100..+100 %), Decontaminate Colors with Amount (0–100 %), and Output To (Selection, Layer Mask, New Layer, New Layer with Layer Mask). It SHALL preview the refined mask and, on OK, apply the settings once: `Selection` replaces the active selection; `Layer Mask` writes the active layer's mask; `New Layer` / `New Layer with Layer Mask` create a layer via copy of the active layer through the refined mask (the latter with a raster mask holding the refined coverage). When Decontaminate Colors is on, `Selection` and `Layer Mask` SHALL be disabled. A successful OK SHALL record exactly one undo state; Cancel SHALL leave the document byte-identical and record nothing.

#### Scenario: The commands are enabled with a document and a selection

- **WHEN** a document is open with a non-empty selection
- **THEN** `Select ▸ Refine Edge…` and the options-bar button are enabled

#### Scenario: OK records one state and refines the selection

- **WHEN** the dialog is opened on a selection and OK is clicked with a positive Radius
- **THEN** the selection changes and the history gains exactly one state

#### Scenario: Cancel is byte-identical

- **WHEN** the dialog is opened and Cancel is clicked
- **THEN** the selection and history are unchanged

#### Scenario: Decontaminate forbids in-place output

- **WHEN** Decontaminate Colors is checked
- **THEN** the Output To choices Selection and Layer Mask are disabled

#### Scenario: Layer Mask output writes the active layer's mask

- **WHEN** Output To is Layer Mask and OK is clicked
- **THEN** the active layer gains a raster mask whose coverage is the refined mask
