# Spec Delta

## MODIFIED Requirements

### Requirement: Layer Style dialog

`Layer ▸ Layer Style ▸ Blending Options…` and each effect entry (`Drop
Shadow…` … `Stroke…`) SHALL open a Layer Style dialog on the Layers panel's
current layer when it can carry a style. The dialog SHALL list Blending Options
then the ten effects in CS6's order (Bevel & Emboss, Stroke, Inner Shadow,
Inner Glow, Satin, Color Overlay, Gradient Overlay, Pattern Overlay, Outer
Glow, Drop Shadow), each effect with a checkbox reflecting `<effect>.on`, and
SHALL show one settings page per row. Opening from an effect entry SHALL show
that page and switch the effect on. Every control SHALL write its value live,
recompositing the canvas without a history state, except the nested contour
editor and gradient editor, which SHALL apply their value to the style when
accepted; their own Cancel SHALL leave the style as it was before the editor
opened. OK SHALL record one "Layer Style" state when anything changed and none
otherwise; Cancel, Escape and the close box SHALL drop every live or applied
edit. The Pattern Overlay page SHALL offer the built-in patterns in a Pattern
menu. A group SHALL open on Blending Options; its effect rows SHALL be visible
but disabled until the compositor renders group effects.

#### Scenario: Opening on an effect and cancelling

- **WHEN** the dialog opens on Color Overlay over a white square on white, Opacity is set to 0, and the dialog is cancelled
- **THEN** the Color Overlay row is current and checked, the square shows red until Opacity reaches 0, and after Cancel the layer has no style and the history count is unchanged

#### Scenario: Pattern Overlay picks a built-in pattern

- **WHEN** the dialog opens on Pattern Overlay over a white square and the Pattern menu is set to its second entry
- **THEN** the row is checked, the menu lists the eight patterns starting with Checkerboard, the checkerboard shows on the square, and the overlay then reads pattern 1

#### Scenario: OK records one state

- **WHEN** the dialog opens on Stroke, Size is set to 4 then 2, and OK is pressed, and an idle dialog is then accepted
- **THEN** exactly one "Layer Style" state is added, a 2 px black stroke shows outside the square, and the idle dialog adds none

## ADDED Requirements

### Requirement: Bevel contour controls

The Bevel & Emboss page SHALL show a Gloss Contour control with its
Anti-aliased box, and a Contour group with a checkbox, a contour control, a
Range control (1–100 %) and an Anti-aliased box, in CS6's order. The contour
control SHALL show the current curve and open the nested contour editor with a
small preset list and a draggable custom curve; the editor's value SHALL apply
when it is accepted. Turning the group's checkbox off SHALL disable the group's
other controls. The page's controls SHALL otherwise write their values live,
recompositing the canvas without a history state; OK SHALL record one "Layer
Style" state and Cancel, Escape and the close box SHALL drop the edits. A group
SHALL show the same page disabled as today.

#### Scenario: A preset contour changes the canvas and Cancel drops it [dialog-contour-editor]

- **WHEN** the Layer Style dialog is open on Bevel & Emboss over a beveled square, a bent contour is picked in the contour editor and accepted, and the Layer Style dialog is then cancelled
- **THEN** the square's bevel shows the bent contour before Cancel and returns to its opening appearance after Cancel, with no history state added

#### Scenario: The gloss contour control writes the effect [dialog-gloss-contour]

- **WHEN** the Gloss Contour control is set to a bent curve and the contour editor is accepted
- **THEN** the bevel's gloss contour reads the same points and the canvas shading differs from the linear contour

#### Scenario: The contour checkbox gates the sub-controls [dialog-contour-gate]

- **WHEN** the Contour checkbox is unchecked
- **THEN** the contour, Range and Anti-aliased controls are disabled and the bevel renders as if the edge contour were off

### Requirement: Gradient Overlay gradient editor

The Gradient Overlay page SHALL replace the two colour swatches with one
gradient control showing the effect's gradient. Opening it SHALL show the shared
Gradient Editor in a solid-only form: the Noise type and the Smoothness control
SHALL be unavailable, stop locations SHALL stay strictly rising, and the colours
SHALL be opaque. Accepting the editor SHALL write the stop list and recomposite;
cancelling it SHALL leave the effect's gradient unchanged. The page's Reverse,
Style, Align With Layer, Angle, Scale and Dither controls SHALL keep their
current behaviour.

#### Scenario: The editor writes the overlay's stops [dialog-gradient-editor]

- **WHEN** the Gradient Overlay page opens, its gradient editor is set to three colour stops, and it is accepted
- **THEN** the swatch shows the new gradient, the effect's stops read the new list, and the canvas recomposites

#### Scenario: Cancelling the editor keeps the gradient [dialog-gradient-cancel]

- **WHEN** the gradient editor is opened and cancelled after changing a stop
- **THEN** the effect's gradient is unchanged

#### Scenario: Stop locations stay strictly rising [dialog-gradient-strict]

- **WHEN** two stops are dragged onto the same location
- **THEN** the editor keeps their locations strictly rising and the accepted list is valid

#### Scenario: The editor is solid-only [dialog-gradient-simple]

- **WHEN** the Gradient Overlay gradient editor opens
- **THEN** its type is Solid, the Noise type and Smoothness control are unavailable, and only a solid gradient can be accepted

#### Scenario: A transparent preset is made opaque [dialog-gradient-opaque]

- **WHEN** the editor's Foreground to Transparent preset is accepted
- **THEN** the effect's stops are written opaque and the swatch matches the written gradient

### Requirement: Channels Include checkboxes

On an RGB document the Blending Options page SHALL show one "Include" checkbox
per working colour plane, checked when the channel is included, after Knockout
and before the group toggles. A document in another colour mode SHALL show no
Include checkboxes in this change. Toggling a checkbox SHALL write that
channel's `blending.includeN` restriction live, recompositing the canvas, and
SHALL work for a group as well as a layer. OK SHALL record the changes in the
single "Layer Style" state and Cancel SHALL drop them.

#### Scenario: Unchecking a channel changes the composite and Cancel restores it [dialog-channel-include]

- **WHEN** the Blending Options page opens on a coloured layer over a differently coloured backdrop, the second channel is unchecked, and the dialog is cancelled
- **THEN** the composite loses that channel's contribution while the dialog is open and returns to its opening appearance after Cancel, with no history state added

#### Scenario: Only RGB shows the channel row [dialog-channel-modes]

- **WHEN** the Blending Options page opens on an RGB, a grayscale and a CMYK document
- **THEN** the RGB dialog shows three Include checkboxes and the grayscale and CMYK dialogs show none

### Requirement: Copy Layer Style captures a channel-only style

`Copy Layer Style` SHALL be enabled when the current layer carries a channel
restriction even without an `lfx2`/`lrFX` effect block, and pasting the copied
style SHALL carry that restriction. `Clear Layer Style` SHALL keep its
effect-block predicate.

#### Scenario: A channel-only style is copyable [dialog-copy-channel-only]

- **WHEN** a layer's only style is an Include restriction
- **THEN** Copy Layer Style is enabled, pasting it carries the restriction, and Clear Layer Style remains disabled
