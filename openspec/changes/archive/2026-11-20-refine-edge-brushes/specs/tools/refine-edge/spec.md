# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
