# ui/layer-style-dialog Specification

## Purpose
The Layer Style dialog: author layer-style effects and their options.

## Requirements

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

### Requirement: Layer Style menu commands

`Copy Layer Style` SHALL be enabled when the current layer has a style and SHALL
copy it to an app-wide style clipboard; `Paste Layer Style` SHALL be enabled
with a copied style and a selection, and SHALL record "Paste Layer Style";
`Clear Layer Style` and `Scale Effects` (a 1–1000 % prompt) SHALL be enabled
when a selected layer has a style and SHALL record "Clear Layer Style" / "Scale
Effects"; `Hide All Effects` / `Show All Effects` SHALL be enabled when some
layer's effects are shown / hidden and SHALL record their own names.
`Global Light…` SHALL be enabled whenever a document is open and SHALL open the
Global Light dialog. `Create Layers` SHALL be enabled when the current layer has
a renderable style and SHALL record one "Create Layers" state. `Rasterize Layer
Style` SHALL be enabled when the current layer's style can be baked (a raster
pixel layer with at least one enabled and present effect, effects not hidden by
the master switch, no customized Blend If or knockout, and no clipped run above),
and SHALL record one "Rasterize Layer Style" state.

#### Scenario: Copy, paste, clear, hide and show

- **WHEN** a red overlay is put on one square, effects are hidden and shown, the style is copied and pasted onto a second square, and the first square's style is cleared and then undone
- **THEN** the overlay disappears and returns, the second square turns red, the first turns white then red again on undo, and each command records its named state

#### Scenario: Global Light and Create Layers are live commands [lsd_menu_global_create]

- **WHEN** a styled layer is current and Global Light… then Create Layers are chosen
- **THEN** the Global Light dialog opens, and Create Layers records one "Create Layers" state and leaves the base layer without a style

#### Scenario: Rasterize Layer Style is enabled only when it can bake [lsd_menu_rasterize]

- **WHEN** the current layer has a renderable style and then only disabled effects
- **THEN** Rasterize Layer Style is enabled in the first case and disabled in the second


### Requirement: Shadow distance and position

The Drop Shadow and Inner Shadow `Distance` controls SHALL range 0–30000 px on
an exponential slider, `v = a·(e^(k·t) − 1)` over the slider fraction `t`, with
61 px at the midpoint and 30000 px at the end; the number field SHALL stay in
step with the slider. While the dialog shows the Drop Shadow or Inner Shadow
page, a left drag on the canvas SHALL move that shadow: the shadow's offset at
the press plus the drag (in image pixels) SHALL set `Distance` and `Angle`
(the light comes from the opposite direction), and the page's controls SHALL
follow. Other pages SHALL take no canvas drag.

#### Scenario: The distance slider is exponential

- **WHEN** the Inner Shadow Distance slider is set to its midpoint
- **THEN** the field and the effect read 61 px, the field's maximum is 30000, and the slider's end is 30000 px

#### Scenario: Dragging on the canvas moves the shadow

- **WHEN** the Drop Shadow page is open and the canvas is dragged straight down by 10 image pixels from a zero offset
- **THEN** the Angle reads 90° and the Distance about 10 px, and on the Satin page the same drag is refused

### Requirement: Canvas zoom keys behind any dialog

While a dialog runs under the shared dialog runner, `Ctrl++` (also `Ctrl+=`,
`Ctrl+Shift+=`), `Ctrl+-`, `Ctrl+0` and `Ctrl+1` SHALL run View ▸ Zoom In,
Zoom Out, Fit On Screen and 100 % on the canvas behind it, unless the dialog's
own shortcuts take the key (a filter preview's zoom keys still zoom the
preview). The Layer Style dialog SHALL run under the shared runner, so the
canvas also takes the wheel and a middle-button pan while it is open.

#### Scenario: Ctrl+= zooms the canvas behind the Layer Style dialog

- **WHEN** the Layer Style dialog runs and Ctrl+= is pressed in it
- **THEN** the canvas zoom increases

### Requirement: Group Blending Options

The Layer Style dialog SHALL open for a group when the row menu's Blending
Options… is chosen. It SHALL open on the Blending Options page. The ten effect
rows SHALL remain visible but disabled, with a not-implemented tooltip, until the
compositor renders group effects.

#### Scenario: A group opens Blending Options only

- **WHEN** Blending Options… is chosen for a group
- **THEN** the dialog opens on page 0 and every effect row is disabled

### Requirement: Global Light dialog

`Layer ▸ Layer Style ▸ Global Light…` SHALL open a dialog holding the document's
master **Angle** (0–359 degrees) and **Altitude** (0–90 degrees), initialised
from the document's current global light. Accepting the dialog SHALL apply both
values to the document, recomposite the canvas, and record exactly one "Global
Light" history state when either value changed; accepting with unchanged values
or cancelling SHALL record nothing. The dialog SHALL refuse no document: it is
enabled whenever a document is open.

#### Scenario: The dialog changes the document light [lsd_global_dialog]

- **WHEN** the Global Light dialog is opened with angle 30 and altitude 30, set to angle 90 and altitude 45, and accepted
- **THEN** the document reads 90/45, every effect with Use Global Light renders from it, and exactly one "Global Light" state is recorded

#### Scenario: Cancelling drops the edit [lsd_global_cancel]

- **WHEN** the Global Light dialog is opened, its angle is changed, and it is cancelled
- **THEN** the document keeps its previous light and no history state is added

### Requirement: Use Global Light in the effect pages

While an effect page's **Use Global Light** checkbox is checked, that page's
Angle control SHALL be disabled and SHALL show the document's global angle; on
the Bevel & Emboss page the Altitude control SHALL likewise show the global
altitude. Unchecking the checkbox SHALL restore the effect's stored local value
and re-enable the control. The stored local value SHALL NOT be overwritten while
the global light is on.

#### Scenario: The angle field follows the global light [lsd_global_page]

- **WHEN** Drop Shadow's Use Global Light is checked and the document's global angle is 90
- **THEN** the Drop Shadow page's Angle field reads 90 and is disabled, and unchecking it restores the effect's stored angle and enables the field

#### Scenario: The bevel altitude field follows the global light [lsd_global_altitude]

- **WHEN** Bevel & Emboss's Use Global Light is checked and the document's global altitude is 60
- **THEN** the bevel Altitude field reads 60 and is disabled, and unchecking it restores the effect's stored altitude and enables the field
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
