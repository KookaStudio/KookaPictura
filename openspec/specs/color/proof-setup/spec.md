# color/proof-setup Specification

## Purpose
Simulate a target output device on screen through View > Proof Setup and Proof
Colors, and expose the active setup to outputs that ask for it, such as
Photoshop PDF's Use Proof Setup.

## Requirements

### Requirement: The active proof setup

The application SHALL keep one active proof setup consisting of a device
profile (Working CMYK, Working Gray, sRGB IEC61966-2.1, Adobe RGB (1998), or
ProPhoto RGB), a rendering intent, a black-point-compensation flag, Simulate
Paper Color, and Simulate Black Ink. The default SHALL be Working CMYK,
relative colorimetric, black-point compensation on, and both simulations off.
The setup SHALL be application view state: it SHALL persist across restarts,
SHALL apply to documents opened later, and MUST NOT be written into the
document, its history, or a saved PSD.

#### Scenario: Defaults are Working CMYK [proof_defaults]

- **WHEN** the application starts with no stored proof setup
- **THEN** Working CMYK is the active setup with relative colorimetric intent, black-point compensation on, and both simulations off

#### Scenario: The setup survives a restart [proof_persist]

- **WHEN** the user selects Working Gray and turns on Proof Colors, then restarts the application
- **THEN** Working Gray is still the active setup and Proof Colors is still on

### Requirement: Soft-proof preview

The colour engine SHALL produce a soft-proof preview of 8-bit sRGB working
pixels that simulates the setup's device profile on the sRGB display using the
setup's rendering intent and black-point compensation. Simulate Paper Color and
Simulate Black Ink SHALL each change the preview when enabled. The preview
SHALL leave alpha unchanged and SHALL return an error, without panicking, for a
device profile that cannot be read. Parity is with Little CMS 2.

#### Scenario: Proofing changes the rendered colours [proof_preview]

- **WHEN** a colour is previewed through Working CMYK, and the same colour is previewed without proofing
- **THEN** the two previewed colours differ and the proofed colour matches the soft-proof transform of the setup

#### Scenario: Each simulation changes the preview [proof_simulations]

- **WHEN** Simulate Paper Color is enabled alone, and Simulate Black Ink is enabled alone
- **THEN** each preview differs from the preview with both simulations off and from the other simulation

#### Scenario: Alpha is untouched [proof_alpha]

- **WHEN** a half-transparent pixel is previewed
- **THEN** its alpha value is unchanged and only its colour changes

### Requirement: View > Proof Setup and Proof Colors

View > Proof Setup SHALL offer Working CMYK, Working Gray, and Custom. Choosing
a built-in SHALL make that setup active and show it checked. Custom SHALL open
the proof setup dialog. View > Proof Colors SHALL be a checkable command bound
to `Ctrl+Y` that toggles the proof display, and its checkmark SHALL follow the
toggle. While proof display is on, the document window title SHALL name the
active proof setup.

#### Scenario: The menu selects a setup [proof_menu]

- **WHEN** the user chooses View > Proof Setup > Working Gray
- **THEN** Working Gray is the active setup and its menu entry is checked while the others are not

#### Scenario: Proof Colors toggles [proof_colors_toggle]

- **WHEN** the user presses `Ctrl+Y`, then presses it again
- **THEN** proof display turns on and the command is checked, then proof display turns off and the check clears

#### Scenario: The title names the proof setup [proof_title]

- **WHEN** Proof Colors is on with Working Gray active
- **THEN** the document window title contains the Working Gray proof setup name

### Requirement: Canvas proof display

While Proof Colors is on, the canvas SHALL draw the visible composite through
the soft-proof preview; while it is off, the canvas SHALL draw the unproofed
composite. Quick Mask and solo-mask views SHALL NOT apply the proof transform.
Proof display SHALL be view-only: it MUST NOT change the document, its
composite, history, or any saved output, except an output that explicitly asks
for the proof setup.

#### Scenario: The canvas shows the proof [proof_canvas]

- **WHEN** a document is displayed with Proof Colors off, then on
- **THEN** the canvas pixels change to the soft-proof preview and change back when it is turned off

#### Scenario: Proofing is view-only [proof_view_only]

- **WHEN** Proof Colors is toggled on and off
- **THEN** the document, its composite, its history, and its modified state are unchanged

#### Scenario: Mask views skip the proof [proof_mask_views]

- **WHEN** a Quick Mask or solo-mask view is displayed with Proof Colors on
- **THEN** the mask view draws its unproofed pixels

### Requirement: Custom proof setup dialog

The Custom dialog SHALL offer Device to Simulate (the five built-in profiles), a
Rendering Intent (Perceptual, Relative Colorimetric, Saturation, or Absolute
Colorimetric), Black Point Compensation, Simulate Paper Color, and Simulate
Black Ink. OK SHALL make the chosen values the active setup; Cancel SHALL leave
the active setup unchanged. Preserve RGB Numbers and Preserve CMYK Numbers are
not modelled.

#### Scenario: Custom applies and Cancel keeps [proof_custom_dialog]

- **WHEN** the Custom dialog is accepted with Simulate Black Ink on, and later opened again and cancelled after changing the device
- **THEN** the first OK enables Simulate Black Ink and the Cancel leaves the device and every other value as they were
