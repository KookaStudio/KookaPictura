# Spec Delta

## ADDED Requirements

### Requirement: Non-opaque solid fills encode and round-trip

`pictura-render` SHALL expose one solid-fill encoder that selects its payload
form from the fill's alpha: `FillContent::Solid([r, g, b, 255])` SHALL be
encoded as the standard version-16 `SoCo` descriptor, and any other alpha SHALL
be encoded as the 4-byte `[r, g, b, a]` `SoCo` tuple. `decode_fill` on the
encoder's output SHALL return the same `FillContent::Solid`, alpha included.
The 4-byte form is in-house: an external PSD reader still opens the file and
classifies the layer as a solid-color fill, but it is not required to decode the
payload, and Photoshop does not represent a translucent solid-color fill.

#### Scenario: A non-opaque fill encodes as the 4-byte tuple

- **WHEN** the encoder is asked for `FillContent::Solid([10, 20, 30, 40])`
- **THEN** the `SoCo` payload is the four bytes `[10, 20, 30, 40]` and `decode_fill` returns `FillContent::Solid([10, 20, 30, 40])`

#### Scenario: An opaque fill still encodes as the descriptor

- **WHEN** the encoder is asked for `FillContent::Solid([10, 20, 30, 255])`
- **THEN** the `SoCo` payload is a version-16 descriptor and `decode_fill` returns `FillContent::Solid([10, 20, 30, 255])`

## MODIFIED Requirements

### Requirement: Solid-color fill layers author the standard descriptor

The app SHALL author a new solid-color fill layer with the payload form that
preserves the colour it was created with: an opaque fill SHALL use the standard
`SoCo` descriptor, while a non-opaque fill SHALL use the in-house 4-byte
`[r, g, b, a]` payload. Both forms SHALL remain readable. A created fill layer
SHALL composite at the requested colour and alpha.

#### Scenario: A new fill layer carries the standard descriptor

- **WHEN** the app adds a solid-color fill layer with alpha 255
- **THEN** its `SoCo` payload is a version-16 descriptor and it composites to the requested colour at full alpha

#### Scenario: A new translucent fill layer keeps its alpha

- **WHEN** a solid-color fill layer is created with the colour `[10, 20, 30, 40]`
- **THEN** its `SoCo` payload is a 4-byte tuple, it decodes to `FillContent::Solid([10, 20, 30, 40])`, and it composites at 40/255 source alpha

#### Scenario: An in-house 4-byte fill still decodes

- **WHEN** a layer carries a 4-byte `SoCo` payload
- **THEN** `decode_fill` returns `FillContent::Solid` with its four stored components

### Requirement: The app can create a solid color fill layer

The app SHALL create a solid-color fill layer carrying the current foreground
colour, and `Layer > New Fill Layer > Solid Color…` and the Layers-panel fill
menu SHALL both offer an enabled `Solid Color…` entry that creates one. The
colour SHALL be read from the app's shared foreground colour state when the
command runs, so a colour chosen in the toolbox, the Color panel, the Swatches
panel, or the Eyedropper is the one the new layer is filled with, and both
creation surfaces create the layer in that same colour. The fill SHALL take
the foreground and never the background, so the default pair fills black and
not the background's white. Because both creation surfaces carry a six-digit
colour payload, the new layer SHALL be opaque whatever the foreground's alpha
is; the alpha-aware encoder preserves alpha when a caller supplies a translucent
colour.

#### Scenario: The panel fill menu uses the foreground [scf_panel_foreground]

- **WHEN** a non-black foreground colour is set and `Solid Color…` is chosen
  from the Layers panel's New Fill / Adjustment strip menu
- **THEN** the new solid-color fill layer composites to that foreground colour

#### Scenario: The Layer menu fill command uses the foreground [scf_menu_foreground]

- **WHEN** a non-black foreground colour is set and `Layer > New Fill Layer >
  Solid Color…` is run
- **THEN** the new solid-color fill layer composites to that foreground colour

#### Scenario: The default foreground still fills black [scf_default_black]

- **WHEN** the foreground colour is the default black and the background colour
  is the default white, and `Solid Color…` is chosen
- **THEN** the new solid-color fill layer is opaque black, not the background
  white
