# adjustment-layer-rendering Specification

## Purpose
Decodes committed adjustment payloads into typed parameters and composites the supported adjustment kinds.

## Requirements

### Requirement: Committed adjustment payloads decode to typed parameters

`decode_adjustment` SHALL decode the committed PSD adjustment payloads into the
matching `pictura_adjust::Adjustment` variant: `expA` to
`Adjustment::Exposure(ExposureParams)`, `vibA` to
`Adjustment::Vibrance(VibranceParams)`, `blwh` to
`Adjustment::BlackWhite(BlackWhiteParams)`, `phfl` to
`Adjustment::PhotoFilter(PhotoFilterParams)`, `grdm` to
`Adjustment::GradientMap(GradientMapParams)`, `blnc` to
`Adjustment::ColorBalance(ColorBalanceParams)`, `mixr` to
`Adjustment::ChannelMixer(ChannelMixerParams)`, `selc` to
`Adjustment::SelectiveColor(SelectiveColorParams)`, `curv` to
`Adjustment::Curves(CurvesParams)`, `SoCo` to
`Adjustment::SolidFill([u8; 4])`, `GdFl` to
`Adjustment::GradientFill(GradientFillParams)`, and `PtFl` to
`Adjustment::PatternFill(PatternFillParams)`. The fixed structs `expA` and
`phfl` SHALL be read as a `u16` version followed by fixed-width fields. The
descriptor payloads `vibA` and `blwh` SHALL be read as a 4-byte descriptor
version equal to 16 followed by a descriptor object whose keys carry the slider
values. A version-2 `phfl` payload SHALL be read as a `u16` colour space, four
`u16` colour components, a `u32` density, and a `u8` luminosity flag; the first
three components SHALL be the filter colour, each in `0..=255`, and the fourth
and the colour space SHALL be ignored. A version-3 `phfl` payload SHALL be read
as three big-endian `u32` CIE XYZ values, a `u32` density, and a `u8` luminosity
flag; the XYZ values SHALL be interpreted as 16.16 fixed-point relative to D50
white and converted to an sRGB filter colour with the same profile-free D50→sRGB
matrix class used for Lab document read; density SHALL be in `0..=100`. A
`grdm` payload SHALL be read as a `u16`
version (`1` or `3`, a `4`-byte method following the two flag bytes when the
version is 3), a `u8` reverse flag, a `u8` dither flag, a unicode gradient name,
and a colour-stop list; each stop SHALL be read as a `u32` location, a `u32`
midpoint, a `u16` mode, and four `u16` colour components whose first three are
reduced from the 16-bit scale to the stop's RGB colour, and the midpoint, mode,
fourth component, and every field after the colour stops SHALL be ignored. A
`blnc` payload SHALL be read as nine big-endian `i16` shifts in the order
shadows, midtones, highlights (three per band, each in `-100..=100`) followed by
a `u8` luminosity flag, and every byte after the flag SHALL be ignored; a
truncated payload or a shift outside `-100..=100` SHALL decode to `None`. A
`mixr` payload SHALL be read as a `u16` version equal to `1`, a `u16`
monochrome flag, then the payload's channels: the `red`, `green`, and `blue`
channels when the flag is clear, and the `gray` channel always. Every channel
SHALL be read as three big-endian `i16` source percentages, two reserved bytes,
and one big-endian `i16` constant percentage, and every byte after the last
channel SHALL be ignored. A non-monochrome payload SHALL decode to
`Adjustment::ChannelMixer` whose `monochrome` is false, whose `red`, `green`,
and `blue` rows are the three channels' source triples, and whose `constant` is
their constants; a monochrome payload SHALL decode with `monochrome` true,
`red` the `gray` source triple, and `constant[0]` the `gray` constant. A `mixr`
payload whose version is not 1, that is shorter than the channels it declares,
or that carries a source percentage or constant outside `-200..=200` SHALL
decode to `None`. A `selc` payload SHALL be read as a `u16` version equal to
`1`, a `u16` correction method (`0` relative, nonzero absolute), and ten plates of
four big-endian `i16` corrections in cyan, magenta, yellow, black order. The first
plate SHALL be ignored without validation (Photoshop reserves it and writes
zeroes); the remaining nine plates SHALL be the ranges reds, yellows, greens,
cyans, blues, magentas, whites, neutrals, and blacks, and every one of their
corrections SHALL be in `-100..=100` or the payload SHALL decode to `None`. Every
byte after the tenth plate SHALL be ignored. A `selc` payload SHALL decode to
`Adjustment::SelectiveColor` with that method and those nine range records. A
`curv` payload SHALL be read as a `u8` ignored byte, a `u16`
version equal to `1`, a `u16` ignored word, and a `u16` channel bitmask (`1`
rgb, `2` red, `4` green, `8` blue); for each set bit, in the order rgb, red,
green, blue, a `u16` node count followed by that many `(i16 output, i16 input)`
pairs. A bitmask equal to zero or carrying a bit outside `0b1111`, a nonzero lead
`is_map` byte, a node count outside `2..=14`, a coordinate outside `0..=255`, or
non-strictly-increasing inputs SHALL decode to `None`. The `2..=14` bound is the
engine-op contract; psd-tools additionally permits up to 19, so a 15–19-point
legacy file SHALL decode to a no-op. The version-4 duplicate `Crv ` section and
every trailing byte SHALL be ignored. A `curv` payload SHALL decode to
`Adjustment::Curves` whose `points` is the rgb channel's points converted from
`(output, input)` to `(input, output)` order and whose `red`, `green`, and
`blue` are the corresponding channels when present; when the rgb bit is clear,
`points` SHALL be the identity endpoints `[(0, 0), (255, 255)]` and only the
per-channel curves SHALL be present. A `SoCo` payload SHALL be read in one of
two forms: a 4-byte straight-alpha RGBA tuple, or a version-16 descriptor whose
`Clr ` object carries `Rd  `, `Grn `, and `Bl  ` `doub` values on the `0..=255`
scale. The descriptor form SHALL decode to `Adjustment::SolidFill([r, g, b,
255])`, rounding and clamping each component to `0..=255` and forcing alpha to
255; the 4-byte form SHALL keep its four stored components. A `GdFl` payload
SHALL be read as a version-16 descriptor whose `Angl` `doub` is the angle in
degrees, whose `Type` enum (typeID `GrdT`) is the gradient kind (`Lnr ` Linear,
`Rdl ` Radial, `Angl` Angle, `Rflc` Reflected, `Dmnd` Diamond), and whose `Grad`
object (class `Grdn`) carries the gradient stops. The `Grad` object's `GrdF`
enum (typeID `GrdF`) SHALL be the custom-stops form (`CstS`); a colour-noise
form (`ClNs`) SHALL NOT decode. Each stop in the `Clrs` list SHALL be an object
whose `Clr ` `RGBC` object carries `Rd  `, `Grn `, and `Bl  ` finite `doub`
values on the `0..=255` scale and whose `Lctn` value is a `doub` or `long` in
`0..=4096`. Stops SHALL be kept in stored order. When present, the top-level
`Rvrs` `bool` SHALL be the reverse flag and the `Scl ` `doub` SHALL be the scale
percent; absent, they SHALL default to false and 100. A `PtFl` payload SHALL be
read as a version-16 descriptor whose `Ptrn` object (class `Ptrn`) carries the
pattern id in its `Idnt` text and whose top-level `Scl ` value (`doub` or unit
float) is the scale percent and whose `Algn` `bool` is the Link With Layer flag;
a missing or wrong-typed `Ptrn`/`Idnt`, a non-finite scale, or a malformed
descriptor SHALL decode to `None`. The pattern id SHALL have trailing NUL bytes
stripped; an absent `Scl ` SHALL default to 100 and an absent `Algn` SHALL
default to true.

#### Scenario: Exposure payload decodes to ExposureParams

- **WHEN** an `expA` payload has version 1 and finite exposure, offset, and gamma values
- **THEN** `decode_adjustment` returns `Adjustment::Exposure` carrying those three values

#### Scenario: Vibrance payload decodes to VibranceParams

- **WHEN** a `vibA` descriptor contains the `vibrance` and `Strt` integer keys in range
- **THEN** `decode_adjustment` returns `Adjustment::Vibrance` with those vibrance and saturation values

#### Scenario: Black and white payload decodes to BlackWhiteParams

- **WHEN** a `blwh` descriptor contains the `Rd  `, `Yllw`, `Grn `, `Cyn `, `Bl  `, `Mgnt`, `useTint`, and `tintColor` keys
- **THEN** `decode_adjustment` returns `Adjustment::BlackWhite` with the six channel percentages, the tint flag, and the tint colour

#### Scenario: Photo Filter payload decodes to PhotoFilterParams

- **WHEN** a `phfl` payload has version 2, four colour components whose first three are each `0..=255`, and a density in `0..=100`
- **THEN** `decode_adjustment` returns `Adjustment::PhotoFilter` whose `color` is the first three components, whose `density` is the stored percent, and whose `preserve_luminosity` is the luminosity byte interpreted as a boolean

#### Scenario: Photo Filter version 3 decodes from XYZ

- **WHEN** a `phfl` payload has version 3, three big-endian `u32` XYZ values, a density in `0..=100`, and a luminosity flag
- **THEN** `decode_adjustment` returns `Adjustment::PhotoFilter` whose `color` is the sRGB conversion of those XYZ values, whose `density` is the stored percent, and whose `preserve_luminosity` is the luminosity byte interpreted as a boolean

#### Scenario: Gradient Map payload decodes to GradientMapParams

- **WHEN** a `grdm` payload has version 1 or 3, a two-flag header, a unicode name, and at least two colour stops whose locations strictly increase within `0..=4096`
- **THEN** `decode_adjustment` returns `Adjustment::GradientMap` whose `stops` carry those locations and the 16-bit-reduced RGB colours and whose `reverse` is the reverse flag interpreted as a boolean

#### Scenario: Gradient Map version 3 skips its method

- **WHEN** a `grdm` payload has version 3 and a `4`-byte method before the name
- **THEN** `decode_adjustment` returns `Adjustment::GradientMap` with the same stops as the equivalent version-1 payload

#### Scenario: Color Balance payload decodes to ColorBalanceParams

- **WHEN** a `blnc` payload carries nine `i16` shifts in `-100..=100`, a luminosity byte, and trailing pad bytes
- **THEN** `decode_adjustment` returns `Adjustment::ColorBalance` whose `shadows`, `midtones`, and `highlights` are the three shifts per band in order and whose `preserve_luminosity` is the luminosity byte interpreted as a boolean

#### Scenario: Channel Mixer payload decodes to ChannelMixerParams

- **WHEN** a non-monochrome `mixr` payload carries version 1, the monochrome flag clear, the `red`/`green`/`blue`/`gray` channels in order, and trailing bytes
- **THEN** `decode_adjustment` returns `Adjustment::ChannelMixer` with `monochrome` false, `red`/`green`/`blue` the three RGB channels' source triples, and `constant` their constants, ignoring the `gray` channel and the trailing bytes

#### Scenario: Monochrome Channel Mixer payload decodes to ChannelMixerParams

- **WHEN** a monochrome `mixr` payload carries version 1, the monochrome flag set, and a `gray` channel
- **THEN** `decode_adjustment` returns `Adjustment::ChannelMixer` with `monochrome` true, `red` the `gray` source triple, and `constant[0]` the `gray` constant

#### Scenario: Composite Curves payload decodes to CurvesParams

- **WHEN** a `curv` payload carries version 1, the bitmask `1`, and an rgb node list in `(output, input)` order
- **THEN** `decode_adjustment` returns `Adjustment::Curves` whose `points` is that list in `(input, output)` order and whose `red`, `green`, and `blue` are `None`

#### Scenario: Per-channel Curves payload decodes to CurvesParams

- **WHEN** a `curv` payload sets the rgb, red, green, and blue bits and carries a node list for each
- **THEN** `decode_adjustment` returns `Adjustment::Curves` whose `points` is the rgb curve and whose `red`, `green`, and `blue` are the corresponding per-channel curves

#### Scenario: Curves without a composite channel decodes to per-channel curves

- **WHEN** a `curv` payload sets only the red, green, or blue bits
- **THEN** `decode_adjustment` returns `Adjustment::Curves` whose `points` is the identity `[(0, 0), (255, 255)]` and whose present per-channel curves carry the stored nodes

#### Scenario: Curves duplicate section and trailing bytes are ignored

- **WHEN** a `curv` payload carries the version-4 duplicate `Crv ` section and pad bytes after its last channel
- **THEN** `decode_adjustment` returns the same `Adjustment::Curves` as the payload without that section

#### Scenario: Selective Color payload decodes to SelectiveColorParams

- **WHEN** a `selc` payload has version 1, a relative method, a zero reserved plate, and nine non-zero range plates
- **THEN** `decode_adjustment` returns `Adjustment::SelectiveColor` with the relative method and those nine ranges in reds through blacks order

#### Scenario: Selective Color reserved plate and trailing bytes are ignored

- **WHEN** a `selc` payload carries non-zero values in its reserved first plate and trailing bytes after the tenth plate
- **THEN** `decode_adjustment` returns the same `Adjustment::SelectiveColor` as the payload with a zero reserved plate and no trailing bytes

#### Scenario: Solid-color fill payload decodes to SolidFill

- **WHEN** a `SoCo` payload is a 4-byte RGBA tuple, or a version-16 descriptor whose `Clr ` object carries `Rd  `/`Grn `/`Bl  ` doubles on the `0..=255` scale
- **THEN** `decode_adjustment` returns `Adjustment::SolidFill` with those components; the descriptor form forces alpha to 255

#### Scenario: Gradient fill payload decodes to GradientFill

- **WHEN** a `GdFl` descriptor has a finite `Angl`, a `Type` kind, and a `Grad` `Grdn` object whose `GrdF` is `CstS` and whose `Clrs` list holds two `RGBC` stops with strictly increasing `Lctn` in `0..=4096`
- **THEN** `decode_adjustment` returns `Adjustment::GradientFill` whose `kind` is the decoded kind, whose `angle_deg` is the `Angl` value, whose `reverse` is false and `scale` is 100 when absent, and whose `stops` carry those locations and colours

#### Scenario: Pattern fill payload decodes to PatternFill

- **WHEN** a `PtFl` descriptor has a `Ptrn` object whose `Idnt` is a pattern id, a finite `Scl `, and an `Algn` flag
- **THEN** `decode_adjustment` returns `Adjustment::PatternFill` whose `pattern_id` is the id with trailing NULs stripped, whose `scale` is the `Scl ` value, and whose `link_with_layer` is the `Algn` value

#### Scenario: Pattern fill defaults are applied

- **WHEN** a `PtFl` descriptor carries a `Ptrn`/`Idnt` but no `Scl ` or `Algn`
- **THEN** the decoded `PatternFill` has `scale` 100 and `link_with_layer` true

#### Scenario: Every gradient fill kind decodes

- **WHEN** the `Type` enum is `Lnr `, `Rdl `, `Angl`, `Rflc`, or `Dmnd`
- **THEN** the decoded `GradientKind` is Linear, Radial, Angle, Reflected, or Diamond respectively

#### Scenario: Malformed gradient fill descriptor is a no-op

- **WHEN** a `GdFl` payload does not parse as a descriptor object, carries a colour-noise `GrdF` (`ClNs`), lacks `Angl`/`Type`/`Grad`/`Clrs`, has fewer than two stops, has non-increasing or out-of-range stop locations, or carries a non-finite angle or colour component
- **THEN** `decode_adjustment` returns `None` and does not panic

#### Scenario: Malformed pattern fill descriptor is a no-op

- **WHEN** a `PtFl` payload does not parse as a descriptor object, lacks a `Ptrn` object, has a missing or wrong-typed `Idnt`, or has a non-finite `Scl `
- **THEN** `decode_adjustment` returns `None` and does not panic

#### Scenario: Malformed exposure payload is a no-op

- **WHEN** an `expA` payload is truncated, has a version other than 1, carries a non-finite value, or has gamma at or below zero
- **THEN** `decode_adjustment` returns `None`

#### Scenario: Malformed vibrance payload is a no-op

- **WHEN** a `vibA` payload is truncated, does not parse as a descriptor object, or carries an out-of-range or wrong-typed key
- **THEN** `decode_adjustment` returns `None`

#### Scenario: Malformed black and white payload is a no-op

- **WHEN** a `blwh` payload is truncated, does not parse as a descriptor object, or carries an out-of-range or wrong-typed key
- **THEN** `decode_adjustment` returns `None`

#### Scenario: Malformed Photo Filter payload is a no-op

- **WHEN** a `phfl` payload is truncated, has a version other than 2 or 3, has a version-2 colour component above 255, or has a density above 100
- **THEN** `decode_adjustment` returns `None`

#### Scenario: Malformed Gradient Map payload is a no-op

- **WHEN** a `grdm` payload is truncated, has a version other than 1 or 3, has fewer than two colour stops, or has non-increasing or out-of-range stop locations
- **THEN** `decode_adjustment` returns `None` and does not panic

#### Scenario: Malformed Color Balance payload is a no-op

- **WHEN** a `blnc` payload is truncated or carries a shift outside `-100..=100`
- **THEN** `decode_adjustment` returns `None` and does not panic

#### Scenario: Malformed Channel Mixer payload is a no-op

- **WHEN** a `mixr` payload has a version other than 1, is shorter than the channels it declares, or carries a source percentage or constant outside `-200..=200`
- **THEN** `decode_adjustment` returns `None` and does not panic

#### Scenario: Malformed Curves payload is a no-op

- **WHEN** a `curv` payload has a version other than 1, a zero bitmask, a bit outside `0b1111`, a node count outside `2..=14`, a coordinate outside `0..=255`, non-increasing inputs, or is truncated mid-channel
- **THEN** `decode_adjustment` returns `None` and does not panic

#### Scenario: Malformed Selective Color payload is a no-op

- **WHEN** a `selc` payload is shorter than 84 bytes, has a version other than 1, or carries a range correction outside `-100..=100`
- **THEN** `decode_adjustment` returns `None` and does not panic

#### Scenario: Malformed solid-color descriptor is a no-op

- **WHEN** a `SoCo` payload is neither a 4-byte RGBA tuple nor a version-16 descriptor object whose `Clr ` object carries `Rd  `, `Grn `, and `Bl  ` finite doubles
- **THEN** `decode_adjustment` returns `None` and does not panic

### Requirement: Descriptor payloads are parsed with the shared codec DOM

The descriptor-based payloads SHALL be parsed with the `pictura-codec`
descriptor DOM. `crates/pictura-codec/src/lib.rs` SHALL expose a public
`read_descriptor(bytes: &[u8]) -> Result<DescValue, PsdError>` built on the
existing descriptor reader, and `pictura-render` SHALL use it rather than a
second descriptor parser. The existing `camera_raw_options` entry point SHALL
continue to decode Pictura Raw `Fltr` options unchanged.

#### Scenario: Descriptor keys are read through the shared DOM

- **WHEN** a `vibA` or `blwh` payload is decoded
- **THEN** its version-16 descriptor is parsed by `pictura-codec::read_descriptor`

#### Scenario: Pictura Raw options still decode

- **WHEN** `pictura_codec::camera_raw_options` is called on a Pictura Raw `Fltr` buffer
- **THEN** it returns the same `DescValue` it returned before this change

#### Scenario: Truncated descriptor is an error, not a panic

- **WHEN** a descriptor payload is truncated
- **THEN** `read_descriptor` returns a `PsdError` and the renderer turns it into `None`

### Requirement: Existing decoded keys are unchanged

The renderer SHALL keep decoding the keys it already understood
(`nvrt`/`invr`, `post`, `thrs`, `brit`, `levl`, `hue2`/`hue `, and the 4-byte
`SoCo`) with their current payload layouts and current `Adjustment` values.

#### Scenario: Existing keys still decode

- **WHEN** an `nvrt`, `post`, `thrs`, `brit`, `levl`, `hue2`, or 4-byte `SoCo` payload is decoded
- **THEN** it yields the same `Adjustment` as before this change

#### Scenario: Unknown key is still a no-op

- **WHEN** a payload key is not a committed or existing key
- **THEN** `decode_adjustment` returns `None` and the composite leaves the backdrop unchanged

### Requirement: A decoded adjustment renders as a non-no-op composite

The renderer SHALL composite a document containing an adjustment layer whose
payload is one of the committed keys without decoding the payload to `None`, and
the resulting backdrop SHALL differ from the same document with the adjustment
layer absent.

#### Scenario: Committed adjustment changes the composite

- **WHEN** a document has an adjustment layer whose payload is a well-formed committed key over a non-uniform backdrop
- **THEN** the composited result differs from the backdrop-only composite

#### Scenario: Masked-out layer stays a no-op

- **WHEN** such a layer's mask hides the whole canvas
- **THEN** the composite equals the backdrop-only composite

### Requirement: Photo Filter payloads encode and round-trip

`pictura-render` SHALL expose `encode_photo_filter(color: [u8; 3], density:
f64, preserve_luminosity: bool) -> AdjustmentData` that builds the version-2
`phfl` block `decode_adjustment` reads. The encoder SHALL clamp `density` to
`0..=100` and clamp each colour component to `0..=255`, so its output always
decodes. `decode_adjustment` on the encoder's output SHALL equal
`Adjustment::PhotoFilter` with the input colour, the clamped density, and the
input luminosity flag.
Encoding SHALL remain version 2 (the app authoring
path); a decoded version-3 payload re-emitted through this encoder becomes
version 2 with the decoded colour.

#### Scenario: Encoded Photo Filter decodes back

- **WHEN** `encode_photo_filter([255, 180, 80], 25.0, true)` is passed to `decode_adjustment`
- **THEN** it returns `Adjustment::PhotoFilter` with `color` `[255, 180, 80]`, `density` `25.0`, and `preserve_luminosity` true

#### Scenario: Out-of-range density is clamped

- **WHEN** `encode_photo_filter` is called with a density above 100 or below 0
- **THEN** the encoded density is clamped into `0..=100` and the block still decodes

#### Scenario: Version-3 open-save re-emits version 2 with decoded colour

- **WHEN** a well-formed version-3 `phfl` decodes and is re-encoded via `encode_photo_filter` using the decoded colour, density, and flag
- **THEN** the new payload has version 2 and `decode_adjustment` returns the same `PhotoFilterParams` colour, density, and flag (up to 8-bit colour quantisation)

### Requirement: The app can create a Photo Filter adjustment layer

The app SHALL map the adjustment kind `photo-filter` to a `phfl` adjustment
layer carrying a warming filter at density 25 with luminosity preservation, and
the Adjustments panel menu SHALL offer a `Photo Filter` entry that dispatches
`adjustment:photo-filter`. Compositing such a layer over a non-uniform backdrop
SHALL change the result and SHALL warm it (red above blue) while keeping the
per-pixel luminance close to the backdrop when luminosity is preserved.

#### Scenario: The photo-filter kind becomes an adjustment layer

- **WHEN** the app adds an adjustment layer of kind `photo-filter`
- **THEN** the new layer carries a `phfl` block and is reported as an adjustment layer

#### Scenario: The panel menu offers Photo Filter

- **WHEN** the Adjustments panel menu is built
- **THEN** it contains a `Photo Filter` row dispatching `adjustment:photo-filter`

#### Scenario: A Photo Filter layer warms and preserves luminosity

- **WHEN** a well-formed Photo Filter adjustment layer is composited over a non-uniform backdrop
- **THEN** the result differs from the backdrop-only composite, red exceeds blue, and each pixel's luminance is within tolerance of the backdrop

### Requirement: Gradient Map payloads encode and round-trip

`pictura-render` SHALL expose `encode_gradient_map(stops: &[GradientStop],
reverse: bool) -> AdjustmentData` that builds the version-1 `grdm` block
`decode_adjustment` reads. The encoder SHALL write the reverse flag, a unicode
name, the supplied colour stops with their 8-bit colours scaled to the 16-bit
storage scale, zero transparency stops, and the trailing gradient fields with
psd-tools' defaults, padded to a 4-byte boundary. `decode_adjustment` on the
encoder's output for a valid stop list SHALL equal
`Adjustment::GradientMap` with the same stops and `reverse` flag.

#### Scenario: Encoded Gradient Map decodes back

- **WHEN** `encode_gradient_map` is called with stops at location 0 colour `[0, 0, 0]` and location 4096 colour `[255, 255, 255]` and `reverse` false
- **THEN** `decode_adjustment` returns `Adjustment::GradientMap` with those two stops and `reverse` false

#### Scenario: The reverse flag round-trips

- **WHEN** `encode_gradient_map` is called with `reverse` true
- **THEN** the decoded `GradientMapParams.reverse` is true

### Requirement: The app can create a Gradient Map adjustment layer

The app SHALL map the adjustment kind `gradient-map` to a `grdm` adjustment
layer carrying a black-to-white gradient with reverse off, and the Adjustments
panel menu SHALL offer a `Gradient Map` entry that dispatches
`adjustment:gradient-map`. Compositing such a layer over a non-uniform backdrop
SHALL change the result by mapping the backdrop's luminance through the
gradient.

#### Scenario: The gradient-map kind becomes an adjustment layer

- **WHEN** the app adds an adjustment layer of kind `gradient-map`
- **THEN** the new layer carries a `grdm` block and is reported as an adjustment layer

#### Scenario: The panel menu offers Gradient Map

- **WHEN** the Adjustments panel menu is built
- **THEN** it contains a `Gradient Map` row dispatching `adjustment:gradient-map`

#### Scenario: A Gradient Map layer changes the composite

- **WHEN** a black-to-white Gradient Map adjustment layer is composited over a non-uniform backdrop
- **THEN** the result differs from the backdrop-only composite

### Requirement: Solid-color fill payloads encode and round-trip

`pictura-render` SHALL expose `encode_solid_color_fill(color: [u8; 3]) ->
AdjustmentData` that builds the standard Photoshop `SoCo` block: a version-16
`DescriptorBlock` whose `Clr ` item is a descriptor object carrying `Rd  `,
`Grn `, and `Bl  ` `doub` values on the `0..=255` scale. `decode_adjustment` on
the encoder's output SHALL equal `Adjustment::SolidFill([r, g, b, 255])`.

#### Scenario: Encoded solid fill decodes back

- **WHEN** `encode_solid_color_fill([10, 20, 30])` is passed to `decode_adjustment`
- **THEN** it returns `Adjustment::SolidFill([10, 20, 30, 255])`

#### Scenario: The encoded block is a version-16 descriptor

- **WHEN** the output of `encode_solid_color_fill` is parsed by `pictura_codec::read_descriptor`
- **THEN** it is an object whose `Clr ` item is an object with the three double components

### Requirement: Solid-color fill layers author the standard descriptor

The app SHALL author a new solid-color fill layer with the standard `SoCo`
descriptor rather than the in-house 4-byte form, while the 4-byte payload SHALL
remain readable for backward compatibility. Because the standard descriptor
carries no alpha, an authored fill layer SHALL be opaque.

#### Scenario: A new fill layer carries the standard descriptor

- **WHEN** the app adds a solid-color fill layer
- **THEN** its `SoCo` payload is a version-16 descriptor and it composites to the requested colour at full alpha

#### Scenario: An in-house 4-byte fill still decodes

- **WHEN** a layer carries a 4-byte `SoCo` payload
- **THEN** `decode_adjustment` returns `Adjustment::SolidFill` with its four stored components

### Requirement: Gradient fill layers render generatively

The renderer SHALL composite a layer whose adjustment decodes to
`Adjustment::GradientFill` by generating an opaque colour for every pixel inside
the layer's rectangle (clamped to the canvas) and blending it through the normal
layer path, so the layer's mask, opacity, fill, and blend mode apply and pixels
outside the rectangle are untouched. Generation SHALL follow the psd-tools
`draw_gradient_fill` geometry for all five kinds — Linear, Radial, Angle,
Reflected, and Diamond — using the params' `angle_deg`, `scale`, and `reverse`
and linearly interpolating the `stops` (clamped outside the stop range). The
generated output alpha SHALL be 255.

#### Scenario: A gradient fill layer changes the composite

- **WHEN** a black-to-white Linear gradient fill layer is composited over a backdrop
- **THEN** the result differs from the backdrop-only composite and the in-rect pixels follow the black-to-white ramp

#### Scenario: Every geometry kind renders

- **WHEN** a gradient fill layer of kind Linear, Radial, Angle, Reflected, or Diamond is composited
- **THEN** each produces its own non-empty in-rect gradient rather than a no-op

#### Scenario: Reverse flips the ramp

- **WHEN** the same black-to-white gradient fill is composited with `reverse` true
- **THEN** the direction of the ramp is inverted relative to the forward case

#### Scenario: Masked-out layer stays a no-op

- **WHEN** a gradient fill layer's mask hides the whole canvas
- **THEN** the composite equals the backdrop-only composite

### Requirement: Gradient fill payloads encode and round-trip

`pictura-render` SHALL expose `encode_gradient_fill(kind: GradientKind, stops:
&[GradientStop], angle_deg: f32) -> AdjustmentData` that builds the standard
Photoshop `GdFl` block: a version-16 `DescriptorBlock` whose `Angl` is the angle,
whose `Type` enum (typeID `GrdT`) is the kind, and whose `Grad` object (class
`Grdn`) carries `GrdF` custom stops (`CstS`), an `Intr` interpolation, and a
`Clrs` list of `RGBC` stops with `Lctn` on the `0..=4096` scale. The encoder
SHALL write only custom-stop gradients and SHALL NOT write `Rvrs`/`Scl `.
`decode_adjustment` on the encoder's output SHALL equal
`Adjustment::GradientFill` with the input kind, stops, and angle (reverse false,
scale 100).

#### Scenario: Encoded gradient fill decodes back

- **WHEN** `encode_gradient_fill(GradientKind::Linear, &[black@0, white@4096], 0.0)` is passed to `decode_adjustment`
- **THEN** it returns `Adjustment::GradientFill` with those stops, kind Linear, `angle_deg` 0.0, `reverse` false, and `scale` 100

#### Scenario: The encoded block is a version-16 descriptor

- **WHEN** the output of `encode_gradient_fill` is parsed by `pictura_codec::read_descriptor`
- **THEN** it is an object whose `Type` enum is the kind and whose `Grad` object carries a `Clrs` list of `RGBC` stops

### Requirement: The app can create a gradient fill layer

The app SHALL create a gradient fill layer for the default kind `gradient-fill`
(black-to-white Linear at angle 0) through a document-sized `GdFl` layer, and
`Layer > New Fill Layer > Gradient…` and the Layers-panel fill menu SHALL both
offer an enabled `Gradient…` entry that creates one. Compositing such a layer
over a backdrop SHALL change the result.

#### Scenario: The gradient-fill kind becomes a fill layer

- **WHEN** the app adds a gradient fill layer
- **THEN** the new layer carries a `GdFl` block, is a fill-content layer, and composites to a black-to-white ramp

#### Scenario: The menus offer Gradient

- **WHEN** the Layer menu and the Layers-panel fill menu are built
- **THEN** each offers an enabled `Gradient…` entry that creates a gradient fill layer

### Requirement: Color Balance payloads encode and round-trip

`pictura-render` SHALL expose `encode_color_balance(shadows: [f64; 3],
midtones: [f64; 3], highlights: [f64; 3], preserve_luminosity: bool) ->
AdjustmentData` that builds the `blnc` block `decode_adjustment` reads: the nine
shifts as big-endian `i16` in shadows/midtones/highlights order, the luminosity
byte, and pad bytes to a 4-byte boundary. The encoder SHALL clamp each shift to
`-100..=100`, so its output always decodes. `decode_adjustment` on the encoder's
output SHALL equal `Adjustment::ColorBalance` with the input shifts and
luminosity flag. The emitted block SHALL match the psd-tools `ColorBalance`
layout, so an independent psd-tools read of the same bytes reports the same
`shadows`, `midtones`, `highlights`, and `luminosity`.

#### Scenario: Encoded Color Balance decodes back

- **WHEN** `encode_color_balance` is called with a midtones shift of `[25.0, 0.0, 0.0]`, the other bands zero, and luminosity preservation true
- **THEN** `decode_adjustment` returns `Adjustment::ColorBalance` with those shifts and `preserve_luminosity` true

#### Scenario: Out-of-range shifts are clamped

- **WHEN** `encode_color_balance` is called with a shift above 100 or below -100
- **THEN** the encoded shift is clamped into `-100..=100` and the block still decodes

#### Scenario: psd-tools reads the encoded block

- **WHEN** the bytes produced by `encode_color_balance` are read by `psd_tools.psd.adjustments.ColorBalance`
- **THEN** its `shadows`, `midtones`, `highlights`, and `luminosity` equal the encoder's inputs

### Requirement: The app can create a Color Balance adjustment layer

The app SHALL map the adjustment kind `color-balance` to a `blnc` adjustment
layer carrying the Photoshop default (all three band shifts zero and luminosity
preservation on), and the Adjustments panel menu SHALL offer a `Color Balance`
entry that dispatches `adjustment:color-balance`. Because the default shifts are
zero, the default layer SHALL leave the backdrop unchanged. A `blnc` layer whose
shifts are not all zero SHALL change the composite over a non-uniform backdrop.

#### Scenario: The color-balance kind becomes an adjustment layer

- **WHEN** the app adds an adjustment layer of kind `color-balance`
- **THEN** the new layer carries a `blnc` block and is reported as an adjustment layer

#### Scenario: The default Color Balance layer is neutral

- **WHEN** a `color-balance` adjustment layer with the default parameters is composited over a backdrop
- **THEN** the result equals the backdrop-only composite

#### Scenario: The panel menu offers Color Balance

- **WHEN** the Adjustments panel menu is built
- **THEN** it contains a `Color Balance` row dispatching `adjustment:color-balance`

#### Scenario: A non-neutral Color Balance layer changes the composite

- **WHEN** a Color Balance adjustment layer with a non-zero midtone shift is composited over a non-uniform backdrop
- **THEN** the result differs from the backdrop-only composite

### Requirement: The document pattern library is decoded from the Patt resource

`pictura-codec` SHALL expose `decode_patterns(&Document) -> Vec<PatternPixels>`
that decodes the document's Patterns resource into row-major RGBA pattern
pixels carrying each pattern's id, width, and height. The resource SHALL be read
from the `Patt`, `Pat2`, and `Pat3` additional-layer-information tagged blocks
in the Layer and Mask Information section (preserved in
`Document.layer_section_extra`), NOT from `Document.image_resources` (image
resource 1039 is the ICC profile). Each block SHALL be parsed as a `Patterns`
list of length-prefixed version-1 patterns: image mode, width/height point,
Unicode name, Pascal pattern id, and a `VirtualMemoryArrayList` whose
`num_channels + 2` channel arrays are each decoded with the existing 8-bit
channel decompression. The written colour channels SHALL be the pattern colour
(the first written channel replicated for a grayscale pattern and the first
three taken as R, G, B for an RGB pattern), and a written channel in the alpha
region SHALL be the pattern's transparency with 255 when absent. Only 8-bit
planes SHALL be decoded: a written channel whose declared depth or pixel depth
is not 8 SHALL cause the pattern to be skipped. An RGB pattern SHALL have
exactly three written colour planes and a grayscale pattern exactly one; a
pattern with a missing or extra colour plane, or whose declared rectangle
disagrees with its written channel rectangles or exceeds a bounded pixel cap,
SHALL be skipped. A malformed or unrecognised block SHALL be skipped, keeping
the patterns parsed so far, and the decoder SHALL NOT panic or allocate without
bound.

#### Scenario: A Patt block decodes to pattern pixels

- **WHEN** a document carries a `Patt` tagged block with a version-1 RGB pattern whose channels hold known 8-bit planes
- **THEN** `decode_patterns` returns one `PatternPixels` with that pattern's id, size, and RGBA pixels in row-major order

#### Scenario: A grayscale pattern replicates its single channel

- **WHEN** a pattern's image mode is Grayscale with one written colour channel
- **THEN** the decoded pixel's R, G, and B components all equal that channel

#### Scenario: A malformed Patt block is skipped

- **WHEN** a `Patt` block is truncated or one pattern's channel list is malformed
- **THEN** `decode_patterns` returns the patterns parsed before the failure and does not panic

#### Scenario: Non-8-bit pattern planes are skipped

- **WHEN** a `Patt` block's written channel declares a depth or pixel depth other than 8
- **THEN** `decode_patterns` skips that pattern and returns the patterns parsed before it, without decoding the plane or panicking

#### Scenario: A crafted oversized pattern rectangle is skipped

- **WHEN** a pattern's declared rectangle is inconsistent with its written channel rectangles or exceeds the decoder's pixel cap
- **THEN** `decode_patterns` skips the pattern without an unbounded allocation and does not panic

#### Scenario: The trailing alpha slot is the pattern transparency

- **WHEN** both slots of a pattern's two-slot alpha region are written
- **THEN** the decoded alpha is the last written slot, matching psd-tools' trailing-plane rule

#### Scenario: A pattern with a missing colour plane is skipped

- **WHEN** an RGB pattern does not have exactly three written colour planes, or a grayscale pattern does not have exactly one
- **THEN** `decode_patterns` skips it rather than filling the absent channel with zero

### Requirement: Pattern fill layers render by tiling the document pattern

`decode_adjustment` SHALL cause the renderer to composite a `PtFl` pattern fill
(`Adjustment::PatternFill`) by tiling the referenced pattern over the layer's
rectangle clamped to the canvas, blending
each in-rect sample at the pattern's alpha through the normal layer path, so the
layer's mask, opacity, fill, and blend mode apply and pixels outside the
rectangle are untouched. The pattern SHALL be resolved by `pattern_id` from the
document's decoded pattern pixels. The tile SHALL use the params' `scale`
(percent; the tile is resampled when the scale is not 100),
`link_with_layer` (the tile is anchored at the layer's top-left when set and at
the document origin when clear) and `origin` (a pixel offset added to the
sample). When no decoded pattern matches `pattern_id`, the layer SHALL composite
a grey placeholder over its rectangle rather than a no-op, and SHALL NOT panic.

#### Scenario: A pattern fill layer tiles its pattern

- **WHEN** a layer references a 2x2 RGB pattern at scale 100 with Link With Layer on and is composited over a backdrop
- **THEN** the in-rect pixels repeat the 2x2 tile and the result differs from the backdrop-only composite

#### Scenario: Adjacent tile cells differ

- **WHEN** the composite is sampled at horizontally adjacent in-rect pixels and again one tile width across
- **THEN** the adjacent pixels carry different tile colours and the pixel one tile width across repeats the first

#### Scenario: Link With Layer anchors the tile to the layer rect

- **WHEN** `link_with_layer` is true and the layer rectangle starts at a non-zero offset
- **THEN** the tile's first cell is aligned with the layer rectangle's top-left, so the pattern moves with the layer

#### Scenario: A missing pattern falls back to the placeholder

- **WHEN** a layer's `pattern_id` does not match any pattern decoded from the document
- **THEN** the in-rect composite is the opaque grey placeholder and the composite is not a no-op

#### Scenario: A masked-out pattern fill is a no-op

- **WHEN** a pattern fill layer's mask hides the whole canvas
- **THEN** the composite equals the backdrop-only composite

### Requirement: Channel Mixer payloads encode and round-trip

`pictura-render` SHALL expose `encode_channel_mixer(monochrome: bool, red:
[f64; 3], green: [f64; 3], blue: [f64; 3], constant: [f64; 3]) ->
AdjustmentData` that builds the `mixr` block `decode_adjustment` reads: a `u16`
version equal to 1, a `u16` monochrome flag, the `red`/`green`/`blue` rows when
not monochrome followed by a `gray` row writing `rgb = red` and
`constant = constant[0]`, or the `gray` row (`rgb = red`,
`constant = constant[0]`) followed by 30 zero
bytes when monochrome, so the block is 44 bytes in both forms. Each channel
SHALL write its three source percentages, two zero reserved bytes, and its
constant. The encoder SHALL clamp each source percentage and constant to
`-200..=200`, so its output always decodes. `decode_adjustment` on the encoder's
output SHALL equal `Adjustment::ChannelMixer` with the input monochrome flag,
rows, and constants (the monochrome rows' unused `green`/`blue` values decoded
from Photoshop defaults).

#### Scenario: Encoded non-monochrome Channel Mixer decodes back

- **WHEN** `encode_channel_mixer(false, [30, -10, 50], [10, 90, 0], [0, 20, 110], [5, -20, 40])` is passed to `decode_adjustment`
- **THEN** it returns `Adjustment::ChannelMixer` whose `monochrome` is false, whose rows are those triples, and whose `constant` is `[5, -20, 40]`

#### Scenario: Encoded monochrome Channel Mixer decodes back

- **WHEN** `encode_channel_mixer(true, [20, 40, 60], [0, 100, 0], [0, 0, 100], [-15, 0, 0])` is passed to `decode_adjustment`
- **THEN** it returns `Adjustment::ChannelMixer` whose `monochrome` is true, whose `red` is `[20, 40, 60]`, and whose `constant[0]` is `-15`

#### Scenario: Out-of-range weights are clamped

- **WHEN** `encode_channel_mixer` is called with any source percentage or constant above 200 or below -200
- **THEN** the encoded value is clamped into `-200..=200` and the block still decodes

### Requirement: Channel Mixer fixtures match an independent ag-psd decoder

The committed fixture `crates/pictura-codec/tests/fixtures/channel_mixer.psd` SHALL carry
a non-monochrome `mixr` block and a monochrome `mixr` block with
fixed values, and a test SHALL read the fixture with the independent `ag-psd` npm
package through `node` and SHALL assert the decoded `monochrome` flag and the
`red`, `green`, `blue`, and `gray` channels equal the authored values. The test
SHALL self-skip with a clear message when `node` or `ag-psd` is unavailable, and
SHALL NOT fail the suite in that case.

#### Scenario: ag-psd reads the non-monochrome fixture block

- **WHEN** the fixture's non-monochrome `mixr` layer is read by ag-psd
- **THEN** `monochrome` is false and its `red`, `green`, and `blue` channels carry the authored triples and constants, and its `gray` channel carries the authored (renderer-ignored) values

#### Scenario: ag-psd reads the monochrome fixture block

- **WHEN** the fixture's monochrome `mixr` layer is read by ag-psd
- **THEN** `monochrome` is true and its `gray` channel carries the authored triple and constant

#### Scenario: The oracle self-skips without ag-psd

- **WHEN** `node` or the `ag-psd` package is not available
- **THEN** the test reports a skip and the suite passes

### Requirement: The app can create a Channel Mixer adjustment layer

The app SHALL map the adjustment kind `channel-mixer` to a `mixr` adjustment
layer carrying the neutral identity default (monochrome off, the rows
`[100,0,0]`/`[0,100,0]`/`[0,0,100]`, constants `[0,0,0]`), and the Adjustments
panel menu SHALL offer a `Channel Mixer` entry that dispatches
`adjustment:channel-mixer`. Because the identity rows leave the matrix
unchanged, the default layer SHALL leave the backdrop unchanged. A `mixr` layer
with a non-identity row SHALL change the composite over a non-uniform backdrop.

#### Scenario: The channel-mixer kind becomes an adjustment layer

- **WHEN** the app adds an adjustment layer of kind `channel-mixer`
- **THEN** the new layer carries a `mixr` block and is reported as an adjustment layer

#### Scenario: The default Channel Mixer layer is neutral

- **WHEN** a `channel-mixer` adjustment layer with the default parameters is composited over a backdrop
- **THEN** the result equals the backdrop-only composite

#### Scenario: The panel menu offers Channel Mixer

- **WHEN** the Adjustments panel menu is built
- **THEN** it contains a `Channel Mixer` row dispatching `adjustment:channel-mixer`

#### Scenario: A non-neutral Channel Mixer layer changes the composite

- **WHEN** a Channel Mixer adjustment layer with a non-identity row is composited over a non-uniform backdrop
- **THEN** the result differs from the backdrop-only composite

### Requirement: Curves payloads encode and round-trip

`pictura-render` SHALL expose `encode_curves(points: &[(u8, u8)], red:
Option<&[(u8, u8)]>, green: Option<&[(u8, u8)]>, blue: Option<&[(u8, u8)]>) ->
AdjustmentData` that builds the `curv` block `decode_adjustment` reads: a `u8`
ignored byte, a `u16` version equal to `1`, a `u16` ignored word, and a `u16`
bitmask setting a bit for each present non-empty channel (`1` rgb, `2` red, `4`
green, `8` blue), followed by each present channel's `u16` node count and its
`(output, input)` pairs in the order rgb, red, green, blue. The encoder SHALL
write only the version-1 bitmask section and SHALL NOT write the version-4
duplicate `Crv ` section. `decode_adjustment` on the encoder's output SHALL
equal `Adjustment::Curves` with the input composite curve as `points`, the input
per-channel curves as `red`/`green`/`blue`, and `None` for every absent channel
(when the rgb channel is absent, `points` decodes as the identity
`[(0, 0), (255, 255)]`).

#### Scenario: Encoded composite Curves decodes back

- **WHEN** `encode_curves(&[(0, 0), (64, 32), (192, 224), (255, 255)], None, None, None)` is passed to `decode_adjustment`
- **THEN** it returns `Adjustment::Curves` whose `points` is those points and whose `red`/`green`/`blue` are `None`

#### Scenario: Encoded per-channel Curves decodes back

- **WHEN** `encode_curves` is called with an rgb curve and all three per-channel curves
- **THEN** `decode_adjustment` returns `Adjustment::Curves` carrying all four curves

#### Scenario: Absent channels are omitted from the bitmask

- **WHEN** `encode_curves` is called with only a composite curve
- **THEN** the encoded bitmask is `1` and no per-channel curve is written

### Requirement: Curves fixtures match an independent ag-psd decoder

The committed fixture `crates/pictura-codec/tests/fixtures/curves.psd` SHALL
carry a composite-only `curv` block and a per-channel `curv` block with fixed
values, and a test SHALL read the fixture with the independent `ag-psd` npm
package through `node` and SHALL assert the decoded `rgb`, `red`, `green`, and
`blue` curves equal the authored points. The fixture SHALL be regenerated
byte-stably by `scripts/generate-fixtures.py`. The test SHALL self-skip with a
clear message when `node` or `ag-psd` is unavailable, and SHALL NOT fail the
suite in that case.

#### Scenario: ag-psd reads the composite fixture block

- **WHEN** the fixture's composite-only `curv` layer is read by ag-psd
- **THEN** its `rgb` curve carries the authored points and its `red`/`green`/`blue` curves are absent

#### Scenario: ag-psd reads the per-channel fixture block

- **WHEN** the fixture's per-channel `curv` layer is read by ag-psd
- **THEN** its `rgb`, `red`, `green`, and `blue` curves carry the authored points

#### Scenario: The oracle self-skips without ag-psd

- **WHEN** `node` or the `ag-psd` package is not available
- **THEN** the test reports a skip and the suite passes

### Requirement: Selective Color payloads encode and round-trip

`pictura-render` SHALL expose `encode_selective_color(method:
SelectiveColorMethod, ranges: &[SelectiveRange; 9]) -> AdjustmentData` that
builds the `selc` block `decode_adjustment` reads: a `u16` version equal to `1`,
a `u16` correction method (`0` relative, `1` absolute), a zero reserved first
plate, and nine range plates in reds, yellows, greens, cyans, blues, magentas,
whites, neutrals, and blacks order, each plate four big-endian `i16` corrections
in cyan, magenta, yellow, black order. The encoder SHALL clamp every correction
to `-100..=100`, so its output always decodes. `decode_adjustment` on the
encoder's output SHALL equal `Adjustment::SelectiveColor` with the input method
and the input nine ranges.

#### Scenario: Encoded Selective Color decodes back

- **WHEN** `encode_selective_color` is called with the relative method and a known nine-range set is passed to `decode_adjustment`
- **THEN** it returns `Adjustment::SelectiveColor` with the relative method and those nine ranges

#### Scenario: Out-of-range corrections are clamped

- **WHEN** `encode_selective_color` is called with a correction above 100 or below -100
- **THEN** the encoded correction is clamped into `-100..=100` and the block still decodes

### Requirement: Selective Color fixtures match an independent ag-psd decoder

The committed fixture `crates/pictura-codec/tests/fixtures/selective_color.psd` SHALL
carry a relative `selc` block and an absolute `selc` block with fixed
nine-range values, and a test SHALL read the fixture with the independent
`ag-psd` npm package through `node` and SHALL assert the decoded method and the
`reds`, `yellows`, `greens`, `cyans`, `blues`, `magentas`, `whites`, `neutrals`,
and `blacks` ranges equal the authored values. The fixture SHALL be regenerated
byte-stably by `scripts/generate-fixtures.py`. The test SHALL self-skip with a
clear message when `node` or `ag-psd` is unavailable, and SHALL NOT fail the
suite in that case.

#### Scenario: ag-psd reads the relative fixture block

- **WHEN** the fixture's relative `selc` layer is read by ag-psd
- **THEN** its mode is relative and its nine named ranges carry the authored corrections

#### Scenario: ag-psd reads the absolute fixture block

- **WHEN** the fixture's absolute `selc` layer is read by ag-psd
- **THEN** its mode is absolute and its nine named ranges carry the authored corrections

#### Scenario: The oracle self-skips without ag-psd

- **WHEN** `node` or the `ag-psd` package is not available
- **THEN** the test reports a skip and the suite passes

### Requirement: The app can create a Selective Color adjustment layer

The app SHALL map the adjustment kind `selective-color` to a `selc` adjustment
layer carrying the neutral default (relative method, all nine ranges zero), and
the Adjustments panel menu SHALL offer a `Selective Color` entry that dispatches
`adjustment:selective-color`. Because every range is zero, the default layer
SHALL leave the backdrop unchanged. A `selc` layer with a non-zero range SHALL
change the composite over a non-uniform backdrop.

#### Scenario: The selective-color kind becomes an adjustment layer

- **WHEN** the app adds an adjustment layer of kind `selective-color`
- **THEN** the new layer carries a `selc` block and is reported as an adjustment layer

#### Scenario: The default Selective Color layer is neutral

- **WHEN** a `selective-color` adjustment layer with the default parameters is composited over a backdrop
- **THEN** the result equals the backdrop-only composite

#### Scenario: The panel menu offers Selective Color

- **WHEN** the Adjustments panel menu is built
- **THEN** it contains a `Selective Color` row dispatching `adjustment:selective-color`

### Requirement: Color Lookup payloads decode to ColorLookupParams

`decode_adjustment` SHALL decode a `clrL` payload into a new
`Adjustment::ColorLookup(ColorLookupParams)`. The payload SHALL be read as a
`u16` version equal to `1`, followed by a version-16 `DescriptorBlock` whose
`lookupType` enum (typeID `3DLUT`) selects a 3-D LUT, `LUTFormat` enum
(`LUTFormatCUBE`, `LUTFormat3DL`, `LUTFormatLOOK`) names the embedded format,
and `LUT3DFileData` carries the embedded LUT file bytes. A payload that is
truncated, carries a version other than `1`, does not parse as a descriptor
object, or lacks a `lookupType` SHALL decode to `None` and SHALL NOT panic.
`ColorLookupParams` SHALL carry the decoded `kind` (3-D LUT, abstract profile,
or device link) and, when `kind` is a 3-D LUT whose format is `.CUBE`, a parsed
`Lut3d` with its cube size and RGB points in the `.CUBE` order (red index
fastest, then green, then blue); if the embedded data does not parse as a valid
`.CUBE` (bad or absent `LUT_3D_SIZE`, size outside `2..=64`, wrong point count,
a non-finite component, or a format other than `.CUBE`), `lookup` SHALL be
`None`. An identity three-dimensional lookup SHALL be the exact identity within
one LSB per channel.

The `dataOrder` and `tableOrder` enums and the `Dthr` dither flag SHALL be read
as block metadata but SHALL NOT change how an embedded `.CUBE` is sampled, since
the `.CUBE` point order is intrinsic to the file.

#### Scenario: A 3-D LUT payload decodes to ColorLookupParams

- **WHEN** a `clrL` payload has version 1 and a descriptor with `lookupType` `3DLUT`, `LUTFormat` `LUTFormatCUBE`, and a valid `LUT_3D_SIZE 2` `LUT3DFileData`
- **THEN** `decode_adjustment` returns `Adjustment::ColorLookup` whose `kind` is 3-D LUT and whose `lookup` is `Some` with size 2 and eight points

#### Scenario: An abstract-profile lookup decodes without a LUT

- **WHEN** a `clrL` payload has `lookupType` `abstractProfile`
- **THEN** `decode_adjustment` returns `Adjustment::ColorLookup` whose `kind` is abstract profile and whose `lookup` is `None`

#### Scenario: Malformed Color Lookup is a no-op

- **WHEN** a `clrL` payload is truncated, has a version other than 1, does not parse as a descriptor object, or lacks `lookupType`
- **THEN** `decode_adjustment` returns `None` and does not panic

#### Scenario: A non-CUBE or malformed embedded LUT decodes without a lookup

- **WHEN** the embedded `LUT3DFileData` is empty, is not a `.CUBE`, declares a `LUT_3D_SIZE` outside `2..=64`, or carries the wrong number of points
- **THEN** `decode_adjustment` returns `Adjustment::ColorLookup` with `lookup` `None` rather than a panic

### Requirement: Color Lookup renders the embedded cube

`apply` SHALL render `Adjustment::ColorLookup` by sampling the parsed `.cube`
trilinearly: it SHALL normalize each pixel channel to `0.0..=1.0`, place it on
the cube's `size`-point grid, take the eight surrounding nodes, and interpolate
with the fractional part; it SHALL write the sampled RGB back to the color
channels and SHALL leave alpha untouched. A `lookup` of `None` (abstract
profile, device link, non-`.CUBE` format, or malformed data) SHALL leave the
buffer bit-exactly unchanged. An identity cube SHALL leave the buffer within one
LSB per channel; a cube with distinct corner colours SHALL map a node exactly to
that node's colour.

Oracle expectation: no ImageMagick operator applies an arbitrary `.cube`, and
Photoshop's `.cube` sampling is not independently reproducible here, so there is
no Adobe pixel-parity claim; known-value tests cover the identity, exact node
mapping (which fixes the red-fastest point order), and a mid-cube trilinear
blend.

#### Scenario: An identity cube is the identity

- **WHEN** Color Lookup is applied with an identity `LUT_3D_SIZE 2` cube to any buffer
- **THEN** every colour channel is within 1 LSB of its input and alpha is bit-identical

#### Scenario: A cube corner maps exactly to its node

- **WHEN** a cube's corner node for a grey input is a distinct colour and the pixel sits exactly on that corner
- **THEN** the output is that node's colour

#### Scenario: The red-fastest point order is honored

- **WHEN** a `LUT_3D_SIZE 3` cube colours only the node at red 1, green 0, blue 0 and a pixel maps exactly to that node
- **THEN** the output is that node's colour, proving the point order is red-fastest

#### Scenario: A missing lookup is a no-op

- **WHEN** `apply` is called with an `Adjustment::ColorLookup` whose `lookup` is `None`
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Alpha is preserved

- **WHEN** Color Lookup is applied to an RGBA buffer
- **THEN** the alpha plane is bit-identical to the input

### Requirement: Color Lookup payloads encode and round-trip

`pictura-render` SHALL expose `encode_color_lookup(file_data: &[u8], name: &str)
-> AdjustmentData` that builds the `clrL` block `decode_adjustment` reads: a
`u16` version equal to `1`, then a version-16 descriptor whose `lookupType` is
`3DLUT`, whose `LUTFormat` is `LUTFormatCUBE`, whose `dataOrder` and `tableOrder`
are `rgbOrder`, whose `Dthr` is false, whose `Nm  ` and `LUT3DFileName` are
`name`, and whose `LUT3DFileData` is `file_data`. `decode_adjustment` on the
encoder's output for a valid `<N> 2..=64` `.CUBE` SHALL equal
`Adjustment::ColorLookup` whose `kind` is 3-D LUT and whose `lookup` carries the
cube's size and points. `pictura-render` SHALL also expose
`identity_cube() -> Vec<u8>` returning a `LUT_3D_SIZE 2` identity `.cube`, so a
default layer renders as a no-op.

#### Scenario: Encoded Color Lookup decodes back

- **WHEN** the bytes of a valid identity `.cube` are passed to `encode_color_lookup` and the block to `decode_adjustment`
- **THEN** it returns `Adjustment::ColorLookup` whose `kind` is 3-D LUT and whose `lookup` has size 2

#### Scenario: The encoded block is a version-1 descriptor

- **WHEN** the output of `encode_color_lookup` is parsed by `pictura_codec::read_descriptor` after the 2-byte version
- **THEN** it is an object whose `lookupType` enum is `3DLUT` and whose `LUT3DFileData` carries the input bytes

### Requirement: Color Lookup fixtures match an independent ag-psd decoder

The committed fixture SHALL carry a `clrL` block at
`crates/pictura-codec/tests/fixtures/color_lookup.psd` whose embedded
`LUT3DFileData` is a known `.cube`,
and a test SHALL read the fixture with the independent `ag-psd` npm package
through `node` and SHALL assert `lookupType`, `lutFormat`, `dataOrder`,
`tableOrder`, `dither`, and the exact `LUT3DFileData` bytes. The fixture SHALL
be regenerated byte-stably by `scripts/generate-fixtures.py`. The test SHALL
self-skip with a clear message when `node` or `ag-psd` is unavailable, and SHALL
NOT fail the suite in that case.

#### Scenario: ag-psd reads the fixture block

- **WHEN** the fixture's `clrL` layer is read by ag-psd
- **THEN** `lookupType` is `3DLUT`, `lutFormat` is `LUTFormatCUBE`, `dataOrder` and `tableOrder` are `rgbOrder`, `dither` is false, and the `LUT3DFileData` bytes equal the authored `.cube`

#### Scenario: The oracle self-skips without ag-psd

- **WHEN** `node` or the `ag-psd` package is not available
- **THEN** the test reports a skip and the suite passes

### Requirement: The app can create a Color Lookup adjustment layer

The app SHALL map the adjustment kind `color-lookup` to a `clrL` adjustment
layer carrying an identity cube, and the Adjustments panel menu SHALL offer a
`Color Lookup` entry that dispatches `adjustment:color-lookup`. Because the
identity cube is exact, the default layer SHALL leave the backdrop unchanged. A
`clrL` layer whose cube is not the identity SHALL change the composite over a
non-uniform backdrop.

#### Scenario: The color-lookup kind becomes an adjustment layer

- **WHEN** the app adds an adjustment layer of kind `color-lookup`
- **THEN** the new layer carries a `clrL` block and is reported as an adjustment layer

#### Scenario: The default Color Lookup layer is neutral

- **WHEN** a `color-lookup` adjustment layer with the default identity cube is composited over a backdrop
- **THEN** the result equals the backdrop-only composite

#### Scenario: The panel menu offers Color Lookup

- **WHEN** the Adjustments panel menu is built
- **THEN** it contains a `Color Lookup` row dispatching `adjustment:color-lookup`

### Requirement: The app can create a solid color fill layer

The app SHALL create a solid-color fill layer carrying the current foreground
colour, and `Layer > New Fill Layer > Solid Color…` and the Layers-panel fill
menu SHALL both offer an enabled `Solid Color…` entry that creates one. The
colour SHALL be read from the app's shared foreground colour state when the
command runs, so a colour chosen in the toolbox, the Color panel, the Swatches
panel, or the Eyedropper is the one the new layer is filled with, and both
creation surfaces create the layer in that same colour. The fill SHALL take
the foreground and never the background, so the default pair fills black and
not the background's white. The `SoCo` descriptor carries no alpha, so the new
layer SHALL be opaque whatever the foreground's alpha is.

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

### Requirement: Full-parameter gradient fill payloads encode and round-trip

`pictura-render` SHALL expose `encode_gradient_fill_params(params:
&GradientFillParams) -> AdjustmentData`. It builds the same version-16 `GdFl`
descriptor as `encode_gradient_fill`, and in addition SHALL write `Rvrs` (bool)
when `params.reverse` is true and `Scl ` (double) when `params.scale` differs
from 100. `decode_adjustment` on its output SHALL equal
`Adjustment::GradientFill(params)`.

#### Scenario: Reverse and scale survive the round-trip [alf_gradient_full_roundtrip]

- **WHEN** `encode_gradient_fill_params` encodes a Radial gradient at angle 45 with reverse true and scale 60
- **THEN** `decode_adjustment` returns those stops, kind, angle, reverse, and scale

#### Scenario: Defaults stay absent [alf_gradient_defaults]

- **WHEN** the params have reverse false and scale 100
- **THEN** the encoded descriptor carries no `Rvrs` or `Scl ` item and decodes to the decode defaults

### Requirement: Pattern fill payloads encode and round-trip

`pictura-render` SHALL expose `encode_pattern_fill(params:
&PatternFillParams) -> AdjustmentData`. It builds a version-16 `PtFl`
descriptor whose `Ptrn` object carries `Idnt` for `params.pattern_id`, plus
`Scl ` for the scale, `Algn` for `link_with_layer`, and a `phase` point when
`params.origin` is not `(0, 0)`. `decode_adjustment` on its output SHALL equal
`Adjustment::PatternFill(params)`.

#### Scenario: A pattern payload decodes back [alf_pattern_roundtrip]

- **WHEN** `encode_pattern_fill` encodes pattern id `"p1"`, scale 50, link false, and origin `(3, 4)`
- **THEN** `decode_adjustment` returns those exact pattern params

#### Scenario: The default origin is omitted [alf_pattern_default_origin]

- **WHEN** the params have origin `(0, 0)`
- **THEN** the encoded descriptor carries no `phase` object and decodes to origin `(0, 0)`
