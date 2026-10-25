# Spec Delta — ui/fill-layer-editor

## ADDED Requirements

### Requirement: Solid Color fill editor

The system SHALL edit a Solid Color fill layer through the application's color
picker, opened either from the layer's content thumbnail or from
`Layer > Layer Content Options…`. Accepting the picker SHALL replace the fill's
RGB content in one undoable state and recomposite the layer; cancelling SHALL
leave the layer, the document, and the history unchanged. The fill remains
opaque; the picker's alpha is not stored.

#### Scenario: A solid fill is edited [fle_solid_apply]

- **WHEN** the user opens the Solid Color fill editor, picks a new colour, and accepts
- **THEN** the layer's `SoCo` fill carries the new colour, the composite updates, and one history state is recorded

#### Scenario: Cancelling leaves the fill unchanged [fle_solid_cancel]

- **WHEN** the user opens the Solid Color fill editor and cancels
- **THEN** the fill, the composite, and the history are unchanged

### Requirement: Gradient Fill editor

The system SHALL edit a Gradient Fill layer through a dialog carrying the
gradient sample, the existing Gradient Editor, Style (Linear, Radial, Angled,
Reflected, Diamond), Angle, Scale, and Reverse. Accepting SHALL replace the
layer's `GdFl` content in one undoable state, preserving all chosen values on a
save/reopen round-trip; cancelling SHALL leave the layer unchanged. Dither and
Align With Layer SHALL be present, disabled, and carry the `— not implemented
yet` tooltip; the renderer always aligns the ramp to the layer rectangle. The
dialog SHALL apply on OK; live on-canvas gradient dragging is not provided.

#### Scenario: A gradient is edited and applied [fle_gradient_apply]

- **WHEN** the user changes the stops, kind, angle, scale, and reverse in the Gradient Fill editor and accepts
- **THEN** the layer's `GdFl` block carries every value, the composite updates, and one history state is recorded

#### Scenario: A gradient edit survives re-encoding [fle_gradient_roundtrip]

- **WHEN** a gradient with reverse true and scale 60 is written and decoded again
- **THEN** the decoded params carry reverse true and scale 60

#### Scenario: The Gradient Editor opens from the sample [fle_gradient_editor]

- **WHEN** the user activates the gradient sample
- **THEN** the Gradient Editor opens on the current gradient and an accepted edit updates the dialog's sample

#### Scenario: Cancelling leaves the gradient unchanged [fle_gradient_cancel]

- **WHEN** the user changes values in the Gradient Fill editor and cancels
- **THEN** the layer's fill, the composite, and the history are unchanged

### Requirement: Pattern Fill editor

The system SHALL edit a Pattern Fill layer through a dialog listing the patterns
decoded from the open document, with Scale, Link With Layer, and Snap To Origin.
Accepting SHALL replace the layer's `PtFl` content in one undoable state;
cancelling SHALL leave the layer unchanged. When the document has no patterns,
the dialog SHALL show an empty state and both the editor and Pattern creation
SHALL be unavailable; the Pattern row SHALL carry a no-patterns reason rather
than `— not implemented yet`. A built-in pattern library and `Edit > Define
Pattern` are outside this capability.

#### Scenario: A pattern fill is edited [fle_pattern_apply]

- **WHEN** the document contains a pattern, the user selects it with a scale and link choice in the Pattern Fill editor, and accepts
- **THEN** the layer's `PtFl` block carries that pattern id, scale, and link flag, the composite updates, and one history state is recorded

#### Scenario: Snap To Origin resets the offset [fle_pattern_snap]

- **WHEN** the user activates Snap To Origin and accepts
- **THEN** the pattern's origin offset is `(0, 0)`

#### Scenario: No patterns disables the editor [fle_pattern_empty]

- **WHEN** the document has no pattern resources
- **THEN** the Pattern Fill editor reports the empty state and Pattern creation and editing are unavailable

### Requirement: Fill creation opens the type editor

`Layer > New Fill Layer` SHALL run the New Layer dialog followed by the chosen
type's editor and SHALL create the layer only when both are accepted. The
Adjustments-panel fill icons and the Layers-panel New Fill / Adjustment menu
SHALL create the layer with the current defaults and then open the type editor;
cancelling that editor SHALL leave the default layer in place. Every successful
creation SHALL insert the layer by the anchored rule, select it, apply the fill
default mask, and record one undo state.

#### Scenario: The Layer menu flow creates once [fle_create_layer_menu]

- **WHEN** the user chooses Layer > New Fill Layer > Gradient…, accepts the New Layer dialog and the Gradient Fill editor
- **THEN** exactly one gradient fill layer is created above the active layer, it is selected, and one history state is recorded

#### Scenario: Cancelling the type editor creates nothing [fle_create_cancel]

- **WHEN** the user accepts the New Layer dialog and then cancels the type editor
- **THEN** no layer and no history state are created

#### Scenario: The panel quick-add opens the editor [fle_create_panel]

- **WHEN** the user clicks the Solid Color icon in the Adjustments panel
- **THEN** a Solid Color fill layer is created with the default content and the Solid Color editor opens on it

### Requirement: Fill editor dispatch

`Layer > Layer Content Options…` SHALL open the type editor for a fill layer and
SHALL keep showing the Properties panel for an adjustment layer. A double-click
on a fill or adjustment layer's content thumbnail SHALL do the same; a
double-click on a mask or vector-mask thumbnail SHALL keep its existing
target/enable behavior, and every other row double-click SHALL keep the Layer
Style path. A non-fill, non-adjustment layer SHALL leave `Layer Content
Options…` disabled.

#### Scenario: Content Options opens the fill editor [fle_content_options]

- **WHEN** a fill layer is current and Layer Content Options is invoked
- **THEN** the matching fill editor opens on that layer

#### Scenario: A fill thumbnail double-click opens the editor [fle_thumb_dblclick]

- **WHEN** the user double-clicks a fill layer's content thumbnail
- **THEN** the matching fill editor opens and no Layer Style dialog opens

#### Scenario: An adjustment thumbnail keeps Properties [fle_adjustment_properties]

- **WHEN** the user double-clicks an adjustment layer's content thumbnail
- **THEN** the Properties panel is shown on that adjustment and no fill editor opens

#### Scenario: A mask thumbnail keeps its action [fle_mask_thumb_unchanged]

- **WHEN** the user double-clicks a fill or adjustment layer's mask thumbnail
- **THEN** the mask thumbnail's existing target behavior runs and no fill editor opens
