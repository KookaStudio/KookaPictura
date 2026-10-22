# tools/fill-engine Specification

## Purpose

The shared fill engine behind `Edit > Fill`, `Edit > Stroke`, the Paint Bucket
and the Gradient: it lays a source colour down over a layer's pixels in a brush
mode, inside the selection, in the document's own colour mode and bit depth.

## ADDED Requirements

### Requirement: The fill engine fills a layer in the layer's own colour space

The fill engine SHALL resolve the target layer's colour planes from the
document's working mode, the shape the compositor reads: three planes (channel
ids 0, 1 and 2) in an RGB working document, or one (channel id 0) in a
Grayscale one. For a Grayscale layer the source colour SHALL be reduced to its
Rec. 601 luma before the blend and the layer's single plane SHALL take the
blended value. For an RGB layer the three planes SHALL take the blended red,
green and blue values.

A Gradient's ramp in a Grayscale document SHALL lie on the gray axis: the
ramp's end colours SHALL be reduced to their Rec. 601 luma before the ramp is
sampled, so the drawn ramp is the interpolation of the luma rather than the
luma of the interpolation.

The fill engine SHALL fill the working copy of a document whose colour mode was
converted on read (CMYK or Lab) and SHALL NOT change its result because of that
mode.

#### Scenario: A Grayscale layer takes the colour's luma [fe_gray_luma]

- **WHEN** a fill with the foreground colour `#DC0000` runs on a Grayscale
  document's layer
- **THEN** the layer's single plane holds 66, the Rec. 601 luma of the colour

#### Scenario: An RGB layer keeps the colour's channels [fe_rgb_channels]

- **WHEN** a fill with the foreground colour `#DC0000` runs on an RGB document's
  layer
- **THEN** the layer's red, green and blue planes hold 220, 0 and 0

#### Scenario: A Grayscale ramp lies on the gray axis [fe_gray_ramp]

- **WHEN** a Gradient between black and white is drawn across a Grayscale
  document's layer
- **THEN** the drawn pixels are gray and the ramp's mid point is the luma of
  the end colours' midpoint, not the luma of the interpolated colour

#### Scenario: A converted colour mode fills on its working copy [fe_converted_8bit]

- **WHEN** a fill runs on a CMYK document
- **THEN** the layer's working RGB planes take the colour and the document's
  retained CMYK planes are left for the save, which re-encodes them

### Requirement: The fill engine writes the pixels the compositor reads

The fill engine SHALL write the target layer's colour planes in the retained
native store when the document has one that is authoritative for the layer —
the document records a source depth of 16 or 32 bits, the document records no
converted colour mode, and the layer's retained store matches the layer's
rectangle and the document's source depth — and the store carries every colour
plane; otherwise it SHALL write the layer's 8-bit colour channels.

A store that is authoritative for only some of the colour planes SHALL refuse
the fill, because the compositor reads the store for the planes it holds, so a
fill of the 8-bit channels would reach only part of the layer.

The transparency plane SHALL be written in the retained store when the store is
authoritative for it, and in the layer's 8-bit channel otherwise.

After a fill that wrote a native plane, the layer's 8-bit channel for that
plane SHALL be the narrowing of the native plane, so the working copy and the
retained store agree. A fill on a native store SHALL blend at the store's own
depth, so its result is not the 8-bit widening of the 8-bit fill's result.

The fill engine SHALL NOT write the retained store's mask plane.

#### Scenario: A depth-16 fill writes the native store [fe_native_store]

- **WHEN** a fill runs on a depth-16 RGB document whose layer retains native
  samples
- **THEN** the layer's retained store holds the fill colour at 16 bits and the
  layer's 8-bit channels hold its narrowing

#### Scenario: A depth-16 fill blends at the store's depth [fe_native_depth]

- **WHEN** a depth-16 fill blends the fill colour over a backdrop whose 16-bit
  value is not 8-bit-aligned
- **THEN** the retained store's sample differs from the 8-bit widening of the
  8-bit fill's result

#### Scenario: A depth-32 fill writes the native store [fe_depth32]

- **WHEN** a fill runs on a depth-32 RGB document whose layer retains native
  samples
- **THEN** the layer's retained store holds the fill colour at 32 bits and the
  layer's 8-bit channels hold its narrowing

#### Scenario: The native blend matches the brush modes [fe_modes]

- **WHEN** a depth-16 fill runs in each brush mode over a half-transparent
  backdrop
- **THEN** the retained store's colour is within one 8-bit step of the 8-bit
  path's colour for the same fill

#### Scenario: The 8-bit channels mirror the store [fe_mirror]

- **WHEN** a fill has written a layer's native store
- **THEN** the layer's 8-bit colour channels equal the narrowing of the store's
  corresponding planes

#### Scenario: A store without a transparency plane still fills [fe_no_alpha_plane]

- **WHEN** a fill runs on a depth-16 document whose layer retains native colour
  samples but no transparency plane
- **THEN** the colour planes take the fill from the native store and the
  transparency comes from the layer's 8-bit channel, exactly as the compositor
  reads it

#### Scenario: A layer without an authoritative store fills the 8-bit channels [fe_fallback]

- **WHEN** a fill runs on a depth-16 document whose layer retains a store that
  does not match the layer's rectangle
- **THEN** the layer's 8-bit channels take the colour

#### Scenario: A depth-16 fill survives a save [fe_round_trip]

- **WHEN** a depth-16 document is filled, saved and read back
- **THEN** the re-read layer's retained samples are the filled ones and are not
  the 8-bit widening of the fill's 8-bit result

### Requirement: The 8-bit fill is unchanged

For a document with no authoritative native store whose layer carries three
colour planes, the fill's result SHALL be byte-identical to the result before
this change: the same pixels, the same rounding, the same brush modes, the same
opaque and transparency behaviour.

#### Scenario: The existing fill behaviour is unmoved [fe_8bit_unchanged]

- **WHEN** the fill engine's 8-bit tests run
- **THEN** they pass with no change to the expected bytes

### Requirement: The Paint Bucket and the Gradient ride the same engine

The Paint Bucket and the Gradient SHALL lay their colour down through the fill
engine, so that in a depth-16 document they write the same pixels `Edit > Fill`
does.

#### Scenario: The tools fill a depth-16 document [fe_consumers]

- **WHEN** a Paint Bucket click and a Gradient drag run on a depth-16 document
- **THEN** the filled pixels take the colour and the ramp in the layer's native
  store, and the canvas shows them

### Requirement: A fill that cannot run is refused

The fill engine SHALL refuse a fill and change no pixel for a group, an
adjustment layer, a layer whose pixels are locked, a transparency-locked layer
in Clear mode, a selection shorter than the canvas, an empty paint, and a zero
opacity. A refused fill SHALL report no changed rectangle.

#### Scenario: A locked layer refuses the fill [fe_refused]

- **WHEN** a fill runs on a layer whose pixels are locked
- **THEN** no pixel changes and the fill reports no changed rectangle
