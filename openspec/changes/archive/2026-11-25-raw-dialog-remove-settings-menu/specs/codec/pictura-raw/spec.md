## REMOVED Requirements

### Requirement: Pictura Raw settings actions

**Reason**: The far-right `fx` settings button and its list are not the Camera
Raw settings menu. The user wants only the Effects tab kept.

**Migration**: None. The actions are removed; the raw open's own OK/Done
settings save is unchanged.

### Requirement: Pictura Raw settings files

**Reason**: The `crs:` XMP Load/Save entries lived only in the removed settings
list.

**Migration**: None. The raw open still reads and writes its settings record on
open and OK/Done.

### Requirement: Pictura Raw effects icon and settings button

**Reason**: The Effects tab keeps the `fx` icon, but the separate settings
button is removed.

**Migration**: None.

## ADDED Requirements

### Requirement: Pictura Raw effects tab

The tab strip SHALL expose eight tabs, with the Effects tab carrying the `fx`
icon and its Grain and Post Crop Vignetting controls. The dialog SHALL NOT show
a separate settings button or settings list at the right of the tab strip.

#### Scenario: The Effects tab is the fx tab [rp_effects_icon]

- **WHEN** the dialog is shown
- **THEN** the tab strip has eight tabs, the Effects tab is the sixth, it carries
  the `fx` icon, and no settings button sits at its right

## MODIFIED Requirements

### Requirement: Pictura Raw panel inset

Every control panel SHALL inset its content from the panel's left and right
edges so a slider's name does not touch the border. The left inset SHALL be at
least twelve pixels.

#### Scenario: Slider names clear the panel border [rp_panel_inset]

- **WHEN** a control panel is shown
- **THEN** its layout carries a left content margin of at least twelve pixels and
  a right content margin of at least eight pixels
