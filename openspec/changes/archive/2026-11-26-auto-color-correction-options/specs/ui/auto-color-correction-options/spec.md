## ADDED Requirements

### Requirement: Auto Color Correction Options for Levels and Curves
The application SHALL offer Auto Options from the Properties menu and from Alt-click on Auto for Levels and Curves adjustment layers. The dialog SHALL provide the three documented algorithms, independent shadow and highlight clip percentages, RGB shadow, midtone, and highlight targets, Snap Neutral Midtones, and Save as Defaults.

#### Scenario: Properties entry points depend on adjustment kind [auto_options_entry]
- **WHEN** the user selects Levels or Curves and opens Auto Options from the Properties menu or Alt-clicks Auto
- **THEN** the same dialog opens with controls initialized from the defaults
- **AND** the action and Auto button are unavailable on unrelated adjustment kinds

#### Scenario: Preview, cancel and accept an adjustment [auto_options_accept]
- **WHEN** the user changes controls
- **THEN** the active Levels or Curves parameters preview the automatic correction
- **AND** waiting longer than the ordinary Properties commit delay does not commit the preview, and cancel restores the opening parameters without adding a state
- **AND** accepting a changed fit creates one undoable Levels or Curves edit, while a normal Auto click uses the saved defaults

### Requirement: Application Auto defaults
The application SHALL persist an accepted Save as Defaults in application preferences. The three direct Image commands SHALL retain their named algorithm and use saved targets and clipping. Only Auto Color SHALL also read the saved snap setting.

#### Scenario: Saved values survive another view and profile read [auto_options_defaults]
- **WHEN** a user accepts with Save as Defaults checked
- **THEN** the dialog initializes from those values on a subsequent open
- **AND** a subsequent acceptance without Save as Defaults or a cancellation does not change the profile

#### Scenario: Configured direct correction respects selection and depth [auto_options_direct]
- **WHEN** a direct Image Auto command runs after saving custom defaults
- **THEN** its fixed algorithm uses the saved target and clip values, and Auto Color also honors the saved snap setting
- **AND** it fits selected source samples, preserves native sample depth and alpha, leaves unselected pixels unchanged, and is undoable once

#### Scenario: Factory results remain unchanged without an override [auto_options_factory]
- **WHEN** any Image Auto command runs with a fresh or legacy profile with no saved override, including after opening or cancelling Options
- **THEN** it produces its previous factory output and does not create an override

#### Scenario: Direct correction retains supported target dispatch [auto_options_dispatch]
- **WHEN** configured Image Auto runs on an eligible native image layer, in Quick Mask, or on a layer mask
- **THEN** the image layer uses authoritative native samples and the existing Quick Mask and layer-mask routes remain supported

### Requirement: Safe behavioral correction fit
The application SHALL derive the correction from the source samples below the Properties adjustment or within the direct command's target. It SHALL stream source-resolution samples for large inputs without averaging away histogram tails, and handle empty and degenerate inputs safely.

#### Scenario: Algorithms, clipping, targets and midtone snap affect the fit [auto_options_fit]
- **WHEN** a controlled sample includes unequal channel ranges, dark and light tails, and a near-neutral midtone
- **THEN** monochromatic mode shares source range, per-channel mode fits independently, and Find Dark & Light samples luminance bands
- **AND** the respective clip and target controls affect the resulting endpoints, with midtone snap using the midtone target
- **AND** an empty or flat sample produces no invalid transfer

#### Scenario: Large composite histogram retains sparse tails [auto_options_tiled_histogram]
- **WHEN** a large composite has known sparse shadow and highlight tails spanning tiles
- **THEN** fitting uses the source-resolution histogram and produces the expected clipped endpoints without a full-size fitting image

### Requirement: Editable adjustment persistence
The fitted Levels and Curves parameters SHALL remain editable and survive PSD and PSB serialization.

#### Scenario: Fitted blocks survive document round trips [auto_options_roundtrip]
- **WHEN** a fitted Levels or Curves adjustment is saved and reopened as PSD and PSB
- **THEN** its parameters and rendered correction are preserved

#### Scenario: Compare visual behavior against CS6 [auto_options_cs6]
- **WHEN** a reference CS6 installation and the same RGB image are available
- **THEN** the two entry points, controls, default persistence, and accept/cancel behavior can be compared side by side

#### Scenario: Brightness and Contrast algorithm availability [auto_options_brightness]

- **WHEN** the user selects Enhance Brightness and Contrast in either Properties dialog
- **THEN** the system SHALL fit a shared luminance-based transfer and disable the target, clipping and neutral-midtones controls while retaining their saved values
- **AND** selecting Find Dark & Light Colors SHALL re-enable those controls

#### Scenario: Curves selected point values [curves_selected_point]

- **WHEN** a point is selected in the Curves Properties graph
- **THEN** the system SHALL show editable input and output values for that point
- **AND** the values SHALL be disabled when no point is selected

#### Scenario: Levels Properties controls [levels_properties_controls]

- **WHEN** a Levels adjustment layer is selected
- **THEN** Properties SHALL show the adjustment input histogram, draggable input and output handles, and black, gray and white point tools
- **AND** Show Clipping SHALL display a clipping preview of the adjustment input

#### Scenario: Curves Properties canvas controls [curves_properties_canvas]

- **WHEN** a Curves adjustment layer is selected
- **THEN** Properties SHALL provide input histogram, endpoint controls, selected point values, on-image adjustment and point samplers using the adjustment input
- **AND** its menu SHALL expose display options, automatic targeting, clipping preview, preset management and reset
