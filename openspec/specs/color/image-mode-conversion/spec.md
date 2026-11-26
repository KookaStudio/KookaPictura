# color/image-mode-conversion Specification

## Purpose
Converts a document between color modes and bit depths (`Image ▸ Mode`,
IMG-004 / IMG-005 / IMG-008) on the 8-bit RGB/Grayscale working model. The
target mode or depth is carried by the retained source stores, so a converted
document saves in its new mode and depth.

## Requirements

### Requirement: Conversion availability follows the IMG-004 matrix

`can_convert_mode(doc, target)` SHALL return false for the document's current
mode, for Duotone and Multichannel, and for an empty document. Bitmap SHALL be
available only from 8-bit Grayscale. Indexed SHALL be available only from 8-bit
Grayscale or RGB. CMYK and Lab SHALL be unavailable from Bitmap and at 32 bits.
RGB SHALL be unavailable from Bitmap. Grayscale SHALL be available from every
other mode. `can_convert_depth(doc, out)` SHALL return false for the current
depth. It SHALL allow every depth for Grayscale/RGB, 8 or 16 for CMYK/Lab, and
none for Bitmap, Indexed, Duotone, or Multichannel. `document_color_mode`
SHALL report the recorded source mode when one is carried, else the working
mode. `document_bit_depth` SHALL report 1 for a Bitmap, else the retained
native depth or 8.

#### Scenario: Bitmap needs Grayscale

- **WHEN** the document is 8-bit RGB
- **THEN** Bitmap is unavailable, and it becomes available after a conversion
  to Grayscale

#### Scenario: High depths dim the 8-bit-only modes

- **WHEN** the document is 16-bit RGB
- **THEN** Indexed and Bitmap are unavailable and CMYK is available; at 32 bits
  CMYK and Lab are unavailable too

### Requirement: Grayscale, RGB, CMYK, and Lab conversions

`convert_mode(doc, target, profiles)` SHALL convert to Grayscale, RGB, CMYK, or
Lab, keep the document's bit depth, and return an error without mutating `doc`
when the conversion is unavailable. `profiles` MAY be absent for a library
caller; the application SHALL always supply the working RGB and working CMYK
profiles from Color Settings.

- Grayscale SHALL replace each composite and layer RGB triple with its
  Rec. 601 luma `(299 R + 587 G + 114 B + 500) / 1000` and leave a single
  color channel. A retained 16/32-bit store SHALL be reduced at its own depth.
- RGB SHALL replicate a gray plane into three, or drop a recorded source mode
  and its stores.
- CMYK SHALL encode each RGB plane set through the working CMYK profile when one
  is supplied, using the working RGB profile as the source and the supplied
  rendering intent and black-point-compensation setting. When no working CMYK
  profile is supplied, CMYK SHALL fall back to the documented profile-free black
  generation (`K = max(R, G, B)`, each ink `round(255 · C / K)` in the stored
  convention). After a profile-aware conversion the document SHALL carry the
  working CMYK profile in image resource `1039`.
- Lab SHALL encode with the profile-free CIELAB transform.

For CMYK and Lab, the encoded planes SHALL be retained as the document's and
each layer's source store, and the working RGB SHALL become what those planes
decode to. Leaving a Bitmap document SHALL first rebuild a Background layer
from its composite.

#### Scenario: Grayscale is a rounded luma

- **WHEN** an RGB layer pixel `(10, 200, 30)` is converted to Grayscale
- **THEN** the layer has one color channel and the pixel is `124`

#### Scenario: A CMYK conversion uses the working profile

- **WHEN** an RGB document is converted to CMYK with Photocraft Coated CMYK as the working CMYK profile
- **THEN** the retained CMYK planes are the lcms2 conversion from the working RGB profile, image resource `1039` carries the working CMYK profile, and a re-read with the profile yields the converted working RGB within tolerance

#### Scenario: A CMYK conversion without a working profile uses the fallback

- **WHEN** a library caller converts an RGB document to CMYK without supplying a working CMYK profile
- **THEN** the retained planes use the documented profile-free black generation and the result remains readable by the profile-free read path

#### Scenario: A CMYK conversion saves as CMYK

- **WHEN** a converted CMYK document is written and read back
- **THEN** the file is header mode CMYK, its planes equal the retained store,
  and the read working RGB equals the converted working RGB within the
  profile round-trip tolerance

### Requirement: Indexed Color conversion

`convert_to_indexed(doc, options)` SHALL first flatten the visible layers into
one Background, discarding hidden layers. It SHALL then build a palette per
`options.reduction` (Exact as Adaptive with 256 entries, Web as the 216
web-safe colors, Local Perceptual/Selective/Adaptive as median cut with at most
`options.colors` entries) and map every pixel to it with the requested dither
and amount. The palette SHALL be retained as `source_palette` and the index
plane as the composite and layer store, with `source_mode` Indexed, so a save
writes header mode Indexed. `indexed_exact_available` SHALL report whether the
composite has at most 256 distinct colors.

#### Scenario: Web palette with diffusion stays on the palette

- **WHEN** an RGB ramp is converted with the Web palette and Diffusion dither
- **THEN** every pixel is one of the 216 web-safe colors and the document reads
  back as Indexed with the same palette

#### Scenario: A layered document is flattened

- **WHEN** a document with a hidden layer and a group is converted to Indexed
- **THEN** it holds exactly one Background layer

### Requirement: Bitmap conversion

`convert_to_bitmap(doc, method)` SHALL flatten an 8-bit Grayscale document and
reduce it to black and white. With 50% Threshold, a value above 128 SHALL be
white and a value at or below 128 SHALL be black. Pattern Dither SHALL use an
8×8 ordered matrix and Diffusion Dither SHALL use Floyd–Steinberg error
diffusion. The result SHALL be a layerless document with no extra channels,
whose packed depth-1 plane (a set bit is black) is the retained store with
`source_mode` Bitmap, so a save writes header mode Bitmap at depth 1.

#### Scenario: Threshold is exact at mid-gray

- **WHEN** gray values `0, 128, 129, 255` are converted with 50% Threshold
- **THEN** they become black, black, white, white, and the document reads back
  as Bitmap

### Requirement: Bit-depth conversion

`convert_bit_depth(doc, out)` SHALL convert 8 → 16/32 and 16 → 32 by building
the missing 8-bit stores (working planes for Grayscale/RGB, encoded planes for
CMYK/Lab) and widening every store (`v · 257`, `v / 255`, `v / 65535`). It
SHALL convert 16 → 8 by dropping a Grayscale/RGB store or narrowing a
CMYK/Lab store to 8 bits. It SHALL record `source_depth`. It SHALL refuse a
32-bit source, which converts through the HDR Conversion tone map.

#### Scenario: 8 to 16 bits saves a 16-bit file

- **WHEN** an 8-bit RGB document is converted to 16 bits and written
- **THEN** it reads back with `source_depth` 16 and each sample is the 8-bit
  value times 257

### Requirement: Save trims the display alpha to three color planes

`save_view(doc)` SHALL return the document with its composite trimmed to three
color planes when the document has an RGB working mode, keeps a four-plane
(RGBA display) composite, and either carries a source mode or carries one or
more document-level extra channels. The trim SHALL apply at every output bit
depth. It SHALL borrow any other document unchanged. The application's PSD/PSB
save SHALL write through it.

**NOTE:** While the document carries extra channels, the written merged image
has no separate transparency plane; the PSD negative-layer-count convention for
merged transparency is not modeled.

#### Scenario: An opened or converted CMYK document re-saves as CMYK

- **WHEN** a CMYK document whose composite the app stored as RGBA is saved
- **THEN** the written file is header mode CMYK, not RGB with an alpha channel

#### Scenario: A four-channel RGB source keeps its channel count [dc_save_alpha_count]

- **WHEN** an RGB document whose source carried three color planes and one extra channel is saved after its composite became four-plane RGBA
- **THEN** the output header declares four channels: three color planes followed by the extra channel

#### Scenario: The extra channel is not duplicated [dc_save_alpha_unique]

- **WHEN** the display composite alpha plane and the extra channel hold different bytes and the document is saved
- **THEN** the written planes are the three color planes and the extra channel, with no duplicated alpha plane

#### Scenario: A depth-16 extra channel keeps its count and depth [dc_save_alpha_depth16]

- **WHEN** a depth-16 RGB document with a four-plane composite, retained native planes, and one extra channel is saved
- **THEN** the output header declares four channels at depth 16, the extra plane is retained, and no alpha plane is duplicated

### Requirement: Halftone Screen Bitmap conversion

Halftone Screen SHALL reduce flattened 8-bit Grayscale to depth-1 Bitmap with a periodic screen at the requested physical frequency and angle. The system SHALL accept 1.000–999.999 lines/inch or 0.400–400.00 lines/cm, angle −180° through 180°, and Round, Diamond, Ellipse, Line, Square, and Cross shapes. Screen spacing SHALL use the document resolution, defaulting to 72 PPI when missing. Black and white input values SHALL remain black and white.

#### Scenario: Halftone settings control a periodic one-bit screen [bitmap_halftone]

- **WHEN** uniform mid-gray is converted at 72 PPI using the Line shape, 9 lines/inch, and 0° or 90°
- **THEN** an eight-pixel screen repeats, the orientation rotates with the angle, and the result retains a packed depth-1 Bitmap plane

#### Scenario: Screen units, shapes, and endpoints are honored [bitmap_screen_geometry]

- **WHEN** screens use equivalent lines/inch and lines/cm settings, each of the six shapes, a non-right angle, or black and white input
- **THEN** equivalent frequencies give the same pixels, each shape follows the specified ranked-cell geometry and rotation, and black and white remain unchanged

#### Scenario: Screen option boundaries are enforced [bitmap_screen_bounds]

- **WHEN** frequency or angle is at a documented boundary, outside its range, or nonfinite
- **THEN** valid boundary values are accepted and invalid values are refused without changing the document

### Requirement: Custom Pattern Bitmap conversion

Custom Pattern SHALL use a selected pattern's grayscale values as spatial thresholds. The system SHALL repeat the tile from the top-left canvas pixel. Black and white input values SHALL remain black and white. The system SHALL decline conversion when the pattern cannot be resolved or has invalid dimensions or samples.

#### Scenario: A chosen pattern repeats across both canvas axes [bitmap_custom]

- **WHEN** a rectangular grayscale tile is used to convert a uniform mid-gray canvas larger than the tile
- **THEN** the tile's lighter and darker thresholds produce a repeating one-bit result across both axes

#### Scenario: Invalid conversion options leave the document unchanged [bitmap_invalid]

- **WHEN** a Halftone Screen option is out of range or a Custom Pattern tile is missing or invalid
- **THEN** the operation SHALL fail without changing the source mode or document

#### Scenario: Missing patterns and endpoint samples are handled [bitmap_pattern_edges]

- **WHEN** the selected pattern is missing or invalid, or black and white input is converted with a valid tile
- **THEN** an unresolved or invalid tile is refused without changing the document, and valid conversion preserves black and white

### Requirement: Bitmap method controls and replay

The Bitmap dialog SHALL offer Halftone Screen and Custom Pattern along with its three existing methods, and SHALL show only the controls applicable to the selected new method. The system SHALL record the selected method and options for silent and modal replay and make a successful conversion one Convert Mode history state. Modal replay SHALL prefill the recorded options, apply accepted edits and return them to the command, and cancel without conversion or changes to the recorded options. Silent replay SHALL refuse an unresolved pattern without changing the document.

#### Scenario: The Bitmap dialog exposes controls for the selected method [bitmap_controls]

- **WHEN** the user switches between Halftone Screen, Custom Pattern, and an existing method
- **THEN** the dialog shows only the selected method's applicable options

#### Scenario: A recorded Bitmap conversion reuses its options [bitmap_replay]

- **WHEN** a recorded conversion with non-default Halftone Screen options or a custom pattern is replayed on an equivalent source
- **THEN** the one-bit output matches the initial conversion and the successful replay adds one Convert Mode history state

#### Scenario: Modal replay pre-fills, edits, and cancels [bitmap_modal_replay]

- **WHEN** a recorded new-method conversion is replayed with its dialog enabled
- **THEN** the dialog is prefilled with recorded options, acceptance applies and returns edited options, and cancellation changes neither document nor recorded options

#### Scenario: Silent replay refuses a missing pattern [bitmap_replay_missing]

- **WHEN** a recorded Custom Pattern conversion is replayed silently and its pattern cannot be resolved
- **THEN** replay fails without changing the document or adding a history state
