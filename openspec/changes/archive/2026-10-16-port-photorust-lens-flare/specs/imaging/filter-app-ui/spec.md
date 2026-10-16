## ADDED Requirements

### Requirement: Lens Flare dialog

`Filter ▸ Render ▸ Lens Flare…` and `Filter ▸ Last Filter Settings` (when the last filter was Lens Flare) SHALL open a dedicated Lens Flare dialog, not the generic slot dialog. The dialog SHALL show a preview of the whole picture, aspect kept and letterboxed, with the flare drawn at the current settings under a crosshair that a click or drag moves. Next to the preview SHALL be OK, Cancel, and a Preview checkbox, on by default. Below the preview SHALL be a Brightness percentage field (10–300 %, default 100) above a slider that stays in step with it, and a `Lens Type` group of four radio buttons (50-300mm Zoom, 35mm Prime, 105mm Prime, Movie Prime; Zoom checked). The dialog SHALL produce the `lens-flare` slots in order: brightness, centre x, centre y, lens index. With Preview on, the canvas SHALL show a non-committing whole-layer preview, deferred during a slider drag until release. OK SHALL commit one history state and Cancel SHALL restore the canvas bit-identically. A Lens Flare canvas preview SHALL always render against the whole layer, never the visible crop.

#### Scenario: The dialog follows CS6's layout and defaults

- **WHEN** the Lens Flare dialog opens on a document
- **THEN** its values are `[100, 0.5, 0.5, 0]`, the Brightness field ranges 10–300 with a `%` suffix, the four lens radios read in CS6's order with Zoom checked, Preview is checked, the buttons sit to the right of the preview, and Brightness and the lens group sit below it

#### Scenario: The preview pad follows the crosshair

- **WHEN** the crosshair is moved to (0.2, 0.3) and then to (0.8, 0.1)
- **THEN** the values carry the new centre, the pad image keeps the picture's aspect, and each time the pad is brightest at the crosshair

#### Scenario: Commit, reopen, and cancel

- **WHEN** the dialog is opened from the Filter menu, the centre is set to (0.25, 0.75), and OK is pressed, and Last Filter Settings is then opened after a `[180, 0.6, 0.4, 2]` commit, and a separate dialog is cancelled after a preview
- **THEN** one history state is added with last params `[100, 0.25, 0.75, 0]` and the flare at that centre, the reopened dialog reads `[180, 0.6, 0.4, 2]`, and the cancelled canvas equals its prior pixels

#### Scenario: The section preview matches the commit

- **WHEN** Lens Flare is previewed with a visible viewport that excludes the flare centre
- **THEN** the preview is rendered against the whole layer and equals the full-layer commit inside the viewport
