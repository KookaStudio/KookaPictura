## ADDED Requirements

### Requirement: Layer Style dialog

`Layer ▸ Layer Style ▸ Blending Options…` and each effect entry (`Drop
Shadow…` … `Stroke…`) SHALL open a Layer Style dialog on the Layers panel's
current layer when it can carry a style. The dialog SHALL list Blending Options
then the ten effects in CS6's order (Bevel & Emboss, Stroke, Inner Shadow,
Inner Glow, Satin, Color Overlay, Gradient Overlay, Pattern Overlay, Outer
Glow, Drop Shadow), each effect with a checkbox reflecting `<effect>.on`, and
SHALL show one settings page per row. Opening from an effect entry SHALL show
that page and switch the effect on. Every control SHALL write its value live,
recompositing the canvas without a history state. OK SHALL record one "Layer
Style" state when anything changed and none otherwise; Cancel, Escape and the
close box SHALL drop every live edit. The Pattern Overlay page SHALL offer the
built-in patterns in a Pattern menu.

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
`Global Light…` and `Create Layers` SHALL stay disabled.

#### Scenario: Copy, paste, clear, hide and show

- **WHEN** a red overlay is put on one square, effects are hidden and shown, the style is copied and pasted onto a second square, and the first square's style is cleared and then undone
- **THEN** the overlay disappears and returns, the second square turns red, the first turns white then red again on undo, and each command records its named state

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
