# Spec Delta

## ADDED Requirements

### Requirement: Modal tool-session undo and redo

The tool framework SHALL provide a transient undo/redo session owned by the
active tool. While a tool's session holds steps, the global Edit Undo and Redo
commands SHALL act on the tool session before the document history, and SHALL
fall back to the document history when the session is empty or no tool session
is active. Recording, walking, or discarding a session step SHALL refresh the
Edit Undo and Redo command enabled state, so their shortcuts work while the
session is the only history. A tool session SHALL be discarded when the tool is
deactivated, cancelled, or applied. Applying a session SHALL commit its result
to the document history as a single state.

#### Scenario: Tool undo takes precedence [ltf_session_precedence]

- **WHEN** a modal tool holds session steps and Edit Undo is invoked
- **THEN** the tool session steps back one step and the document history is unchanged

#### Scenario: Fallback to document history [ltf_session_fallback]

- **WHEN** Edit Undo is invoked with an empty tool session or no modal tool active
- **THEN** the document history is undone

#### Scenario: The Undo command is enabled by the session [ltf_session_enables_undo]

- **WHEN** a modal tool records its first session step and the document history is empty
- **THEN** the Edit Undo command is enabled and invoking it walks the session

#### Scenario: Cancel discards the session [ltf_session_cancel]

- **WHEN** a modal tool is cancelled
- **THEN** its session is discarded and no session step is reachable

#### Scenario: Apply commits one state [ltf_session_apply_one]

- **WHEN** a modal tool with several session steps is applied
- **THEN** exactly one document history state is added

### Requirement: Crop options-bar controls

The Crop tool's options bar SHALL present, left to right and separated into
groups by vertical rules: the tool icon; an aspect-ratio select whose first and
default entry is `Ratio` (a free custom aspect) and whose other entries include
the CS6 presets plus `New Crop Preset...` and `Delete Crop Preset...`, `W` and
`H` numeric fields, and a swap-aspect button; a `Clear` control that resets the
ratio and a spirit-level button that toggles a straighten line mode; a grid
overlay menu button and a settings (cog) menu button; `Delete Cropped Pixels` and
a disabled `Content-Aware` placeholder; a reset button; and cancel and apply
buttons. Every icon button (swap-aspect, spirit-level, grid, cog, reset, cancel,
apply) SHALL use a Lucide SVG icon, not a text glyph. In ratio mode the `W`/`H`
fields SHALL show the aspect-ratio values and editing them SHALL set a custom
ratio; only the `W x H x Resolution` entry SHALL show the pixel width/height and
a resolution field with a unit. `New Crop Preset...` SHALL add the current ratio
to the select; `Delete Crop Preset...` SHALL remove the selected user preset. The
spirit-level toggle SHALL arm a straighten mode in which a pressed, dragged,
released line sets the crop angle to the line's inclination and then disarms.
Applying a preset or field SHALL keep the crop box and any active straighten
consistent. The cog menu SHALL offer a `Use Classic Mode` toggle (checked by
default); a Classic drag moves the crop box, a Modern drag repositions the
composite behind a fixed box, and the mode also decides the initial state
(Classic boxless, Modern a centered preview box). The crop options (mode, grid
overlay, ratio, and Delete Cropped Pixels) SHALL persist across restarts.

#### Scenario: Crop option buttons use Lucide icons [ltf_crop_icons]

- **WHEN** the Crop options bar is shown
- **THEN** the swap-aspect, spirit-level, grid, cog, reset, cancel, and apply buttons each carry a Lucide SVG icon and no text glyph

#### Scenario: Classic vs Modern drag mode [ltf_crop_mode]

- **WHEN** `Use Classic Mode` is toggled in the cog menu and the active box is dragged from inside
- **THEN** Classic moves the box and Modern repositions the composite behind the fixed box

#### Scenario: The mode sets the initial box [ltf_crop_mode_init]

- **WHEN** the Crop tool is selected with Modern off (Classic) vs on
- **THEN** Classic shows no box until one is drawn, and Modern shows a centered preview box immediately

#### Scenario: The ratio menu and resolution mode [ltf_crop_ratio_menu]

- **WHEN** a ratio is chosen with `Ratio` or a preset vs `W x H x Resolution`
- **THEN** the `W`/`H` fields show the bare aspect-ratio values in ratio mode and the pixel width/height plus a resolution field and unit only in resolution mode

#### Scenario: Crop options persist [ltf_crop_persist]

- **WHEN** a crop option is changed and the app restarts
- **THEN** the mode, grid overlay, ratio, and Delete Cropped Pixels are restored to the changed values

#### Scenario: A preset locks the box ratio [ltf_crop_preset]

- **WHEN** a ratio preset is selected in the Crop options bar
- **THEN** the crop box is fitted to that ratio

#### Scenario: A user preset is added and removed [ltf_crop_preset_crud]

- **WHEN** `New Crop Preset...` is used with a ratio and later `Delete Crop
  Preset...` removes it
- **THEN** the select gains and then loses that entry

#### Scenario: A ratio default is the free Ratio entry [ltf_crop_ratio_default]

- **WHEN** the Crop options bar is shown for the first time or after a reset
- **THEN** the aspect-ratio select shows the `Ratio` entry

#### Scenario: Crop control groups are separated [ltf_crop_separators]

- **WHEN** the Crop options bar is shown
- **THEN** its control groups are separated by vertical rules

#### Scenario: Content-Aware is a disabled placeholder [ltf_crop_content_aware]

- **WHEN** the Crop options bar is shown
- **THEN** a `Content-Aware` checkbox is present and disabled

#### Scenario: W and H set the ratio [ltf_crop_fields]

- **WHEN** the `W` or `H` ratio field is committed
- **THEN** the crop box is fitted to that width:height ratio

#### Scenario: Straighten draws a line [ltf_crop_straighten]

- **WHEN** the spirit-level toggle is armed and a line is dragged on the canvas
- **THEN** the crop angle becomes the line's inclination and the mode disarms

### Requirement: Active tool icon separator

Every active tool's options page SHALL draw a vertical separator immediately after
its tool icon, before its other controls, matching the Crop page.

#### Scenario: Every page separates the tool icon [ltf_tool_icon_separator]

- **WHEN** any tool's options bar is shown
- **THEN** a vertical rule separates the tool icon from the next control

### Requirement: Move options-bar controls

The Move tool's options bar SHALL offer an `Auto-Select` choice of Group or Layer,
a `Show Transform Controls` checkbox, the align and distribute button sets, and a
three-dots menu listing the align and distribute actions plus an `Align To:`
choice of Selection or Canvas. Choosing `Align To: Canvas` SHALL align to the
canvas bounds; `Selection` SHALL align to the active selection.

#### Scenario: Align To Canvas [ltf_move_align_canvas]

- **WHEN** `Align To: Canvas` is chosen and an align action is invoked
- **THEN** the selected layers are aligned to the canvas bounds

#### Scenario: Auto-Select and transform controls are present [ltf_move_controls]

- **WHEN** the Move options bar is shown
- **THEN** the `Auto-Select`, `Show Transform Controls`, align, distribute, and three-dots controls are present

### Requirement: Eyedropper options-bar controls

The Eyedropper tool's options bar SHALL offer a `Sample Size` (Point Sample
through the N by N average sizes up to 101 by 101), a `Sample` scope (Current
Layer, Current & Below, All Layers, All Layers No Adjustments, Current & Below
No Adjustments), and a `Show Sampling Ring` checkbox. A sample SHALL average the
chosen size and read the chosen scope.

#### Scenario: Sample size and scope are honored [ltf_eyedropper_sample]

- **WHEN** a sample size and scope are chosen and a pixel is sampled
- **THEN** the foreground is the average of that size read from that scope

#### Scenario: Sampling ring toggles [ltf_eyedropper_ring]

- **WHEN** `Show Sampling Ring` is checked and the eyedropper hovers the canvas
- **THEN** a sampling ring is drawn around the pointer
