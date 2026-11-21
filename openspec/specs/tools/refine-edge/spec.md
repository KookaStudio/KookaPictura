# tools/refine-edge Specification

## Purpose
Refine Edge: edge estimation and the refined selection output.

## Requirements

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

The app SHALL implement `Select ▸ Refine Edge…` (`Ctrl+Alt+R`) and a `Select and Mask…` button in the selection tools options bar, both opening the non-modal `RefineEdgeDialog`. The dialog SHALL expose View Mode (Marching Ants, Overlay, On Black, On White, Black & White, On Layers, Reveal Layer), Show Edge (`J`, the CS6 Show Radius view), Show Original, Smart Radius, Radius (0–100 px), Smooth (0–100), Feather (0–100 px), Contrast (0–100 %), Shift Edge (−100..+100 %), Decontaminate Colors with Amount (0–100 %), and Output To (Selection, Layer Mask, New Layer, New Layer with Layer Mask). It SHALL preview the refined mask and, on OK, apply the settings once: `Selection` replaces the active selection; `Layer Mask` writes the active layer's mask; `New Layer` / `New Layer with Layer Mask` create a layer via copy of the active layer through the refined mask (the latter with a raster mask holding the refined coverage). When Decontaminate Colors is on, `Selection` and `Layer Mask` SHALL be disabled. A successful OK SHALL record exactly one undo state; Cancel SHALL leave the document byte-identical and record nothing.

The dialog SHALL also expose the interactive refinement brush: a Refine Edge Brush that expands the edge-estimation band, and a Restore Original mode (the CS6 Erase Refinements) that reverts the input coverage under the stroke. Alt SHALL be a momentary Restore Original modifier: holding it during a drag restores and releasing it returns to refine unless `Shift+E` has latched Restore Original, in which case the latched mode stays restore. `Shift+E` SHALL latch or unlatch the Restore Original mode. Brush size SHALL be a diameter in document pixels, default 30 px, adjustable from 1 to 1000 px, and respond to the `[` and `]` keys; the dab radius SHALL be half the size and the hover ring diameter SHALL be the size. While the dialog is open it SHALL arm the brush on the document canvas: hovering SHALL draw the size ring, and a left-button drag SHALL add dabs to the preview without changing the document, spacing them a quarter of the dab radius. `J` SHALL toggle the Show Edge band. The strokes SHALL be committed only on OK, and Cancel after any strokes SHALL remain byte-identical.

#### Scenario: The commands are enabled with a document and a selection

- **WHEN** a document is open with a non-empty selection
- **THEN** `Select ▸ Refine Edge…` and the options-bar button are enabled

#### Scenario: OK records one state and refines the selection

- **WHEN** the dialog is opened on a selection and OK is clicked with a positive Radius
- **THEN** the selection changes and the history gains exactly one state

#### Scenario: Cancel is byte-identical

- **WHEN** the dialog is opened, brush strokes are drawn on the canvas, and Cancel is clicked
- **THEN** the selection, layer masks, pixels, and history are unchanged and no preview overlay or brush ring remains

#### Scenario: Decontaminate forbids in-place output

- **WHEN** Decontaminate Colors is checked
- **THEN** the Output To choices Selection and Layer Mask are disabled

#### Scenario: Layer Mask output writes the active layer's mask

- **WHEN** Output To is Layer Mask and OK is clicked
- **THEN** the active layer gains a raster mask whose coverage is the refined mask

#### Scenario: A canvas drag adds brush dabs to the preview [re_brush_canvas]

- **WHEN** the dialog is open and a left-button drag crosses the selection edge on the canvas
- **THEN** the dialog records dabs, the refined preview changes in the brushed area, the document itself is untouched, and OK commits exactly one state

#### Scenario: The brush keys toggle, resize, and show the band [re_brush_keys]

- **WHEN** `Shift+E` is pressed, the bracket keys are pressed, and `J` is toggled while the dialog is open
- **THEN** the restore mode latches and unlatches, the brush size and hover ring change between 1 and 1000 px, and the Show Edge band appears or disappears on the canvas

#### Scenario: Holding Alt restores only while held [re_brush_alt]

- **WHEN** the dialog is open, Alt is held during a drag and then released, a second drag follows without Alt, and `Shift+E` is then latched for a third drag
- **THEN** the first drag's dabs restore the original selection, the second drag's dabs refine, the third drag restores because the mode is latched, and the document is unchanged

### Requirement: Refine Edge distances are document pixels

Boundary distances used by the Radius band and Shift Edge SHALL be in document
pixels. The 3-4 chamfer's orthogonal step SHALL be the divisor, so a pixel one
column from a straight edge reports 1 and a pixel four columns away reports 4.

#### Scenario: Distance is the pixel count [re_distance_pixels]

- **WHEN** a straight edge is measured at 1 and 4 columns away
- **THEN** the reported distances are 1 and 4, and a radius of 4 reaches only
  that far

### Requirement: Refine Edge preview scales with the proxy

The preview SHALL scale Radius, Smooth, and Feather by the preview proxy's
linear shrink factor before refining, so the proxy shows the edge width the
commit will apply. Contrast and Shift Edge are percentages and SHALL NOT scale.

#### Scenario: Preview parameters scale with the proxy [re_preview_scale]

- **WHEN** a large document is previewed at a reduced proxy size
- **THEN** Radius, Smooth, and Feather are scaled by the proxy's linear factor
  and Contrast and Shift Edge are unchanged

#### Scenario: Preview and commit agree within proxy tolerance [re_preview_commit_agree]

- **WHEN** the same edge is previewed at the proxy size and committed at document
  size with the same Radius
- **THEN** the preview's edge transition matches the committed edge transition
  scaled to the proxy within one proxy pixel

### Requirement: Refine Edge brush strokes

The refiner SHALL accept ordered brush dabs alongside the settings, each a disk with an explicit radius and a refine or restore mode. The engine SHALL apply each dab in the pixel space of the coverage it is given. Within a refine dab's disk the effective edge-band radius SHALL be the larger of the slider radius and the dab radius. Within a restore dab's disk the coverage SHALL revert to the input mask's coverage before the global refinements, so with Smooth, Feather, Contrast, and Shift Edge at their defaults the restore-only area SHALL equal the input. Each dab SHALL overwrite the local refinement state in its disk in order, so a later refine dab SHALL replace an earlier refine dab's radius and a later restore dab SHALL clear refinement, including band expansion. The refinement-band output SHALL NOT mark a pixel whose last dab was a restore. A dab SHALL be ignored when any coordinate or the radius is non-finite, the radius is negative, the centre lies outside the coverage, or the radius exceeds the coverage's longer side. The result SHALL remain deterministic.

Strokes SHALL be recorded in document pixels. The preview SHALL scale a dab's `x` by the proxy's x ratio (proxy width / document width), its `y` by the proxy's y ratio, and its radius by the larger of the two ratios before refining, and the commit SHALL pass them unchanged, so preview and commit show the same stroke. The refinement-band output (Show Edge, the CS6 Show Radius view) SHALL mark the pixels the effective radius reaches, including radius expanded by brush dabs.

#### Scenario: A refine dab changes only its own area [re_brush_refine]

- **WHEN** a refine dab covers a soft boundary with Radius 0 and every other refinement at its default
- **THEN** coverage changes inside the dab's disk and is byte-identical to the input everywhere else

#### Scenario: A restore dab reverts and later dabs win [re_brush_restore]

- **WHEN** a refine dab is followed by an overlapping restore dab, and then by another refine dab over part of the same area, with Smooth, Feather, Contrast, and Shift Edge at their defaults
- **THEN** coverage under the restore-only area equals the input mask, and coverage under the final refine dab reflects refinement again

#### Scenario: A later refine dab replaces the earlier radius [re_brush_order]

- **WHEN** a large refine dab is followed by a smaller overlapping refine dab, and then by a restore dab over part of that area
- **THEN** the smaller refine dab replaces the earlier radius in its disk, the restore dab clears refinement including band expansion where it last applies, and the refinement-band output does not mark a pixel whose last dab was a restore

#### Scenario: Invalid dabs are ignored [re_brush_invalid]

- **WHEN** dabs carry a non-finite coordinate or radius, a negative radius, a centre outside the coverage, or a radius larger than the coverage's longer side
- **THEN** those dabs are ignored and the result matches the same run without them

#### Scenario: Brush changes stay bounded [re_brush_bounded]

- **WHEN** a refine dab of radius R is applied with a slider radius S
- **THEN** no pixel changed by edge estimation lies farther than the larger of S and R from the input 50 % contour

#### Scenario: Brush strokes scale with the preview proxy [re_brush_scale]

- **WHEN** the same strokes are previewed at the proxy size and committed at document size with a dab radius of at least 8 document pixels
- **THEN** the preview's brushed edge matches the committed edge scaled to the proxy within two proxy pixels, allowing for the preview scaling's clamp to at least one proxy pixel

#### Scenario: Show Edge marks the brush-expanded band [re_brush_band]

- **WHEN** the refinement-band output is requested with a slider radius and brush dabs
- **THEN** it marks exactly the pixels whose effective radius reaches them, including those reached only because of a refine dab
