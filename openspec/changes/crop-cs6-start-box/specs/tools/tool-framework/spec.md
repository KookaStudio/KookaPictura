## REMOVED Requirements

### Requirement: Crop options-bar controls

**Reason**: It required a disabled `Content-Aware` placeholder and a boxless
Classic start. Content-Aware crop is not a CS6 feature (it arrived in CC
2015.5), and CS6 frames the image with a handled box in either mode.

**Migration**: Replaced by `Crop options-bar layout`, which carries the same
controls without the checkbox and starts both modes in the preview box.

## ADDED Requirements

### Requirement: Crop options-bar layout

The Crop tool's options bar SHALL present, left to right and separated into
groups by vertical rules: the tool icon; an aspect-ratio select whose first and
default entry is `Ratio` (a free custom aspect) and whose other entries include
the CS6 presets plus `New Crop Preset...` and `Delete Crop Preset...`, `W` and
`H` numeric fields, and a swap-aspect button; a `Clear` control that resets the
ratio and a spirit-level button that toggles a straighten line mode; a grid
overlay menu button and a settings (cog) menu button; `Delete Cropped Pixels`; a
reset button; and cancel and apply buttons. There SHALL be no `Content-Aware`
checkbox: Content-Aware crop is not a CS6 feature. Every icon button (swap-aspect, spirit-level, grid, cog, reset, cancel,
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
composite behind a fixed box; both modes start with the centered preview box. The crop options (mode, grid
overlay, ratio, and Delete Cropped Pixels) SHALL persist across restarts.

#### Scenario: Crop option buttons use Lucide icons [ltf_crop_icons]

- **WHEN** the Crop options bar is shown
- **THEN** the swap-aspect, spirit-level, grid, cog, reset, cancel, and apply buttons each carry a Lucide SVG icon and no text glyph

#### Scenario: Classic vs Modern drag mode [ltf_crop_mode]

- **WHEN** `Use Classic Mode` is toggled in the cog menu and the active box is dragged from inside
- **THEN** Classic moves the box and Modern repositions the composite behind the fixed box

#### Scenario: The mode sets the initial box [ltf_crop_mode_init]

- **WHEN** the Crop tool is selected with Modern off (Classic) vs on
- **THEN** both show a centered preview box with its handles immediately

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

#### Scenario: No Content-Aware checkbox [ltf_crop_content_aware]

- **WHEN** the Crop options bar is shown
- **THEN** it has no `Content-Aware` checkbox

#### Scenario: W and H set the ratio [ltf_crop_fields]

- **WHEN** the `W` or `H` ratio field is committed
- **THEN** the crop box is fitted to that width:height ratio

#### Scenario: Straighten draws a line [ltf_crop_straighten]

- **WHEN** the spirit-level toggle is armed and a line is dragged on the canvas
- **THEN** the crop angle becomes the line's inclination and the mode disarms
