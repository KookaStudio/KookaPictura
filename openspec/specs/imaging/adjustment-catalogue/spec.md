# imaging/adjustment-catalogue Specification

## Purpose
Defines the engine-owned adjustment catalogue. It is the one list of adjustment
and fill-layer kinds with their titles and the surfaces each appears on, plus the
parameter specs that drive every adjustment dialog, so the menus, the Adjustments
panel and the dialogs cannot drift from the engine.

## Requirements

### Requirement: The engine owns one adjustment catalogue

The engine SHALL publish one catalogue of adjustment and fill-layer kinds in CS6
menu order. Each entry SHALL carry:

- its kind id;
- its CS6 title;
- its Image-menu command: a dialog under Image > Adjustments, a direct command
  under Image > Adjustments, a direct command at the Image menu root, or none;
- whether Layer > New Adjustment Layer (or New Fill Layer) offers it, and whether
  that leaf carries an ellipsis;
- its Adjustments panel tile icon id, if it has a tile;
- its Adjustments panel menu label, if it has a menu row.

The catalogue SHALL cover:

- the sixteen dialog adjustments;
- the six direct commands: Invert, Desaturate and Equalize under Image >
  Adjustments plus Auto Tone, Auto Contrast and Auto Color at the Image root;
- the three fill layers, Solid Color, Gradient and Pattern.

Shadows/Highlights SHALL be a dialog only, with no adjustment-layer entry. Invert
SHALL be both a direct command and an adjustment layer without an ellipsis. Every
Image-menu and layer-menu entry SHALL name a command that exists in the command
catalogue.

HDR Toning, Match Color and Replace Color are image commands with their own
dialogs and no adjustment layer. They SHALL stay outside the catalogue.

#### Scenario: Catalogue covers every kind once [adjust_catalogue_complete]

- **WHEN** the catalogue is enumerated
- **THEN** every kind id appears exactly once. The dialog entries are the sixteen
  Image > Adjustments dialogs in CS6 order, with Color Lookup after Channel Mixer.
  Each dialog kind has a default adjustment block.

#### Scenario: Layer entries exclude Shadows/Highlights [adjust_catalogue_layer_rows]

- **WHEN** the New Adjustment Layer entries are listed in catalogue order
- **THEN** they are the sixteen CS6 layer adjustments with Color Lookup and Invert
  and without Shadows/Highlights, and only Invert lacks an ellipsis

#### Scenario: Catalogue entries resolve to commands [adjust_catalogue_commands]

- **WHEN** each entry's Image-menu path and layer-menu path is looked up in the
  command catalogue
- **THEN** every path resolves to a command id. The Auto entries resolve at the
  Image root.

### Requirement: Adjustment surfaces build from the catalogue

The shell SHALL hold no kind or title table of its own. It SHALL build the
following from the engine catalogue:

- the handlers on the Image > Adjustments and Image-root commands;
- the Layer > New Adjustment Layer handlers;
- the Layers panel's New Fill/Adjustment menu;
- the Adjustments panel tiles and panel menu rows.

The order, labels, ellipses, command ids and icon ids of these menus and tiles
SHALL stay as they were before the move.

Fill-layer entries supply labels and order only. Their actions stay the existing
fill commands, and Pattern stays disabled or inert.

#### Scenario: Image adjustment commands are wired from the catalogue [adjust_menu_unchanged]

- **WHEN** the main window wires the Image menu
- **THEN** every catalogue entry with an Image-menu command has its command marked
  implemented with a handler, and the Image > Adjustments leaves and order equal
  the existing command-tree expectations

#### Scenario: New Adjustment Layer menus are unchanged [adjust_layer_menu_unchanged]

- **WHEN** Layer > New Adjustment Layer and the Layers panel's New Fill/Adjustment
  menu are built
- **THEN** their leaves and order equal the catalogue's layer entries, and
  choosing an adjustment entry adds that adjustment layer

#### Scenario: Adjustments panel tiles and menu follow the catalogue [adjust_panel_tiles]

- **WHEN** the Adjustments panel and its panel menu are built
- **THEN** the tiles are the catalogue's tile entries in order with their icon
  ids, and the menu's adjustment rows are the catalogue's menu labels in order.
  Clicking an adjustment tile adds that kind, and the Pattern tile stays inert.

### Requirement: Adjustment parameter ranges come only from the engine

Every adjustment dialog, including the generic slider dialog and the hand-laid
Exposure, Hue/Saturation, Levels, Curves, Color Balance, Black & White, Photo
Filter, Channel Mixer and Gradient Map dialogs, SHALL take each parameter
control's minimum, maximum, default and decimals from the engine's parameter
layout for that kind. A dialog SHALL NOT hard-code a numeric range or default the
layout already defines. A dialog's control layout stays in the shell.

Three kinds of control are exempt and keep their shell rules:

- controls whose limits are coupled to a sibling at run time, such as the Levels
  input black/white points kept two levels apart;
- point editors that are not layout parameters, such as the Curves point
  Input/Output fields;
- values derived from another control, such as the Black & White tint hue and
  saturation taken from the tint colour.

#### Scenario: Generic dialog ranges match the layout [adjust_dialog_ranges_generic]

- **WHEN** the Brightness/Contrast or Vibrance dialog opens
- **THEN** each slider's range and default equal the engine layout's spec for that
  parameter (for example Brightness −150…150 and Contrast −50…100)

#### Scenario: Hand-laid dialog ranges match the layout [adjust_dialog_ranges_handlaid]

- **WHEN** each hand-laid adjustment dialog opens on a fresh document
- **THEN** every non-exempt numeric control's range, default and decimals equal
  the engine layout's spec for the parameter it edits
