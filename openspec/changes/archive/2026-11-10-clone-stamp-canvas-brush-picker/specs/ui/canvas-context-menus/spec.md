## MODIFIED Requirements

### Requirement: Paint-tool canvas menu

When the Brush, Pencil, Clone Stamp, Pattern Stamp, History Brush, or Art History
Brush tool is active, a canvas right-click SHALL open the Brush Preset picker,
exposing Size, Hardness, and the preset grid. This menu is a deliberate product
decision, not CS6 parity: the corpus documents the picker as an options-bar
control and the on-canvas size/hardness gesture as the `Alt+Right-drag` HUD.

#### Scenario: Open the brush picker on the canvas

- **WHEN** the Brush tool is active and the user right-clicks over the document
- **THEN** the Brush Preset picker opens at the pointer

#### Scenario: A preset chosen from the canvas applies

- **WHEN** the user picks a preset in the canvas-opened picker
- **THEN** the active paint tip becomes that preset

#### Scenario: Every stamp tool opens the brush picker [ccm_stamp_open]

- **WHEN** the Clone Stamp, Pattern Stamp, History Brush, or Art History Brush is
  active and the user right-clicks over the document
- **THEN** the Brush Preset picker opens at the pointer for that tool

#### Scenario: A preset chosen for a stamp tool applies [ccm_stamp_apply]

- **WHEN** the user picks a preset in the picker opened by a stamp tool
- **THEN** the active paint tip for that stamp tool becomes that preset
