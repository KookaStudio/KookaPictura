# Spec Delta

## MODIFIED Requirements

### Requirement: Color Replacement engine

`pictura_paint::replace` SHALL apply a Color Replacement dab to a layer's
pixels: a pixel under the brush tip SHALL be replaced only when its largest
per-channel difference from the reference colour is within the tolerance
(0–255), fading linearly from 70 % of the tolerance to zero when anti-aliased.
The reference SHALL be the pixel under the dab centre (Continuous), the first
dab's centre pixel (Once), or the background colour (Background Swatch).
Contiguous and Find Edges SHALL replace only pixels connected to the dab centre
through matching pixels, and Find Edges SHALL NOT cross a normalised luminance
step above 0.35. The replacement SHALL apply the selected CS6 paint mode's W3C
blend formula to the foreground, keep the pixel's alpha, never paint a
transparent pixel, and fully replace a pixel at most once per stroke. `Behind`
SHALL leave every pixel unchanged, and `Clear` SHALL erase matched pixels to
transparency (the one mode that does not keep the pixel's alpha).

#### Scenario: Color keeps the shading

- **WHEN** a Color-mode dab replaces a grey ramp with red
- **THEN** both ends turn red and the darker end stays darker

#### Scenario: Contiguous stops at a gap

- **WHEN** a dab straddles two matching stripes separated by a non-matching one
- **THEN** Contiguous leaves the far stripe unchanged and Discontiguous replaces it

#### Scenario: Once keeps the first sample

- **WHEN** a Once stroke starts on green and moves onto red
- **THEN** the green is replaced and the red is unchanged

#### Scenario: Behind and Clear [cr_behind_clear]

- **WHEN** a Behind dab covers opaque pixels and a Clear dab covers matched
  opaque pixels
- **THEN** Behind leaves every pixel byte-for-byte unchanged and Clear sets the
  matched pixels' alpha to zero

### Requirement: Color Replacement tool

The Color Replacement tool SHALL paint a live per-dab stroke on the active
pixel layer and record exactly one "Color Replacement Tool" history state per
stroke that changed pixels. Alt-click SHALL sample the foreground colour. The
options bar SHALL offer Size, Hardness, Mode (the 25-mode CS6 paint list,
default Color), Sampling (default Continuous), Limits (default Contiguous),
Tolerance (0–100 %, default 30 %), and Anti-alias (default on).

#### Scenario: Repaint a field

- **WHEN** a green brush is dragged across a blue field beside a yellow one
- **THEN** one "Color Replacement Tool" state is recorded, the stroked blue
  turns green, and the yellow is unchanged

## ADDED Requirements

### Requirement: Color Replacement mode list

The Color Replacement Mode menu SHALL offer the 25-mode CS6 paint list in this
order: Normal, Dissolve, Behind, Clear, Darken, Multiply, Color Burn, Linear
Burn, Lighten, Screen, Color Dodge, Linear Dodge (Add), Overlay, Soft Light,
Hard Light, Vivid Light, Linear Light, Pin Light, Hard Mix, Difference,
Exclusion, Hue, Saturation, Color, Luminosity. `Color` SHALL remain the default.
Each mode SHALL use its W3C blend formula, except `Behind`, which is a no-op
because replacement never touches transparency, and `Clear`, which erases
matched pixels. A 32-bit document SHALL offer only the modes the 32-bpc paint
contract keeps: Normal, Dissolve, Darken, Multiply, Lighten, Linear Dodge (Add),
Difference, Hue, Saturation, Color, and Luminosity; an 8- or 16-bit document
SHALL offer all 25.

#### Scenario: The menu lists the CS6 paint modes [cr_mode_menu]

- **WHEN** an RGB document is active and the Color Replacement tool is selected
- **THEN** its Mode menu lists the 25 modes from Normal through Luminosity in
  the documented order with Color selected, and every index round-trips through
  the engine

#### Scenario: Normal and Multiply apply their formulas [cr_normal_multiply]

- **WHEN** the same mid-grey pixel and red foreground are blended under Normal
  and under Multiply
- **THEN** Normal yields the foreground colour and Multiply yields the
  per-channel product

#### Scenario: A 32-bit document keeps the subset [cr_32bit_subset]

- **WHEN** a 32-bit RGB document is active
- **THEN** the Mode menu offers only the 11 allowed modes, and Color Burn is not
  selectable

### Requirement: Color Replacement document availability

The Color Replacement tool SHALL be unavailable on Bitmap, Indexed, and
Multichannel documents. `begin_color_replacement` SHALL refuse to start a
stroke, the tool's toolbox flyout row SHALL be disabled, and an active Color
Replacement tool SHALL switch to Move when a document switch or mode conversion
makes it unavailable. The app SHALL NOT convert the document to make the tool
available. RGB, Grayscale, CMYK, Lab, and Duotone documents SHALL keep the tool
available.

#### Scenario: Bitmap disables the tool [cr_bitmap_unavailable]

- **WHEN** a Bitmap document is active
- **THEN** the Color Replacement flyout row is disabled, `setActiveTool` does not
  select it, and the bridge refuses to begin a stroke

#### Scenario: Indexed disables the tool [cr_indexed_unavailable]

- **WHEN** an RGB document with the Color Replacement tool active is converted
  to Indexed
- **THEN** the tool switches to Move and the flyout row is disabled

#### Scenario: Multichannel disables the tool [cr_multichannel_unavailable]

- **WHEN** the document mode name is `multichannel`
- **THEN** the shared availability predicate refuses Color Replacement

#### Scenario: RGB keeps the tool available [cr_rgb_available]

- **WHEN** an RGB document is active
- **THEN** the flyout row is enabled and `setActiveTool` selects Color
  Replacement
