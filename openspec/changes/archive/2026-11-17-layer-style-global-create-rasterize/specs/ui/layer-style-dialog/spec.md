# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
