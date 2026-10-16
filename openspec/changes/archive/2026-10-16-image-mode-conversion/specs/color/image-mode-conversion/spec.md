# color/image-mode-conversion Specification

## Purpose
Converts a document between color modes and bit depths (`Image ▸ Mode`,
IMG-004 / IMG-005 / IMG-008) on the 8-bit RGB/Grayscale working model. The
target mode or depth is carried by the retained source stores, so a converted
document saves in its new mode and depth.

## ADDED Requirements

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

`convert_mode(doc, target)` SHALL convert to Grayscale, RGB, CMYK, or Lab,
keep the document's bit depth, and return an error without mutating `doc` when
the conversion is unavailable.

- Grayscale SHALL replace each composite and layer RGB triple with its
  Rec. 601 luma `(299 R + 587 G + 114 B + 500) / 1000` and leave a single
  color channel. A retained 16/32-bit store SHALL be reduced at its own depth.
- RGB SHALL replicate a gray plane into three, or drop a recorded source mode
  and its stores.
- CMYK SHALL encode each RGB plane set with black generation (`K = max(R, G,
  B)`, each ink `round(255 · C / K)` in the stored convention).
- Lab SHALL encode with the profile-free CIELAB transform.

For CMYK and Lab, the encoded planes SHALL be retained as the document's and
each layer's source store, and the working RGB SHALL become what those planes
decode to. Leaving a Bitmap document SHALL first rebuild a Background layer
from its composite.

#### Scenario: Grayscale is a rounded luma

- **WHEN** an RGB layer pixel `(10, 200, 30)` is converted to Grayscale
- **THEN** the layer has one color channel and the pixel is `124`

#### Scenario: A CMYK conversion saves as CMYK

- **WHEN** a converted CMYK document is written and read back
- **THEN** the file is header mode CMYK, its planes equal the retained store,
  and the read working RGB equals the converted working RGB

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

### Requirement: Source-mode documents save from three color planes

`save_view(doc)` SHALL return the document with its composite trimmed to three
color planes when it carries a source mode, has an RGB working mode, and keeps a
four-plane (RGBA display) composite. It SHALL borrow any other document
unchanged. The application's PSD/PSB save SHALL write through it.

#### Scenario: An opened or converted CMYK document re-saves as CMYK

- **WHEN** a CMYK document whose composite the app stored as RGBA is saved
- **THEN** the written file is header mode CMYK, not RGB with an alpha channel
