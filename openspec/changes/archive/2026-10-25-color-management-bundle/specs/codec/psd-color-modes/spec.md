# Spec Delta

## MODIFIED Requirements

### Requirement: Non-RGB color modes open by normalizing to the working mode

`read_psd` SHALL accept header color modes Bitmap (0), Grayscale (1), Indexed
(2), RGB (3), CMYK (4), Multichannel (7), Duotone (8), and Lab (9). It SHALL convert a non-RGB mode's color
planes into the engine's working mode (RGB for Bitmap/Indexed/CMYK/Lab,
Grayscale unchanged) and SHALL set the returned document's `mode` to `Rgb` or
`Grayscale` and its `depth` to `Eight`. For CMYK, the conversion SHALL use the
embedded resource `1039` profile when it parses as a CMYK profile; otherwise the
working CMYK profile supplied by the caller when one is available; otherwise the
documented profile-free fallback. A header color mode of Duotone (8) or Multichannel (7)
with header channel count 1 or 3 at depth 8 SHALL open by normalizing to the
working mode (Duotone and 1-channel Multichannel as grayscale; 3-channel
Multichannel as profile-free CMY→RGB). Multichannel with any other channel
count, any other mode code, and any bit depth other than 8 (other than depth 1
for Bitmap, and depths 16/32 for the Grayscale/RGB/CMYK/Lab modes read per
`psd-bit-depth`) SHALL return `PsdError::Unsupported`. A header channel count
below the mode's color-channel count SHALL return `PsdError::Invalid`.

#### Scenario: An Indexed document opens as RGB with palette colors

- **WHEN** an 8-bit Indexed PSD whose indices select distinct palette entries is read
- **THEN** `read_psd` returns a document whose `mode` is `Rgb` and whose composite pixels are the palette colors for those indices

#### Scenario: A CMYK document opens as RGB

- **WHEN** an 8-bit CMYK PSD carrying a decodable CMYK resource `1039` is read
- **THEN** `read_psd` returns a document whose `mode` is `Rgb`, whose `depth` is `Eight`, and whose composite is the lcms2 CMYK-to-RGB conversion of the stored planes through that profile

#### Scenario: An untagged CMYK document opens through the working profile

- **WHEN** an 8-bit CMYK PSD with no resource `1039` is opened by the application with a working CMYK profile
- **THEN** the composite is the lcms2 conversion through the working CMYK profile

#### Scenario: An untagged CMYK document with no working profile uses the fallback

- **WHEN** an untagged CMYK PSD is read by the library convenience with no working CMYK profile
- **THEN** the composite uses the documented profile-free formula

#### Scenario: A Lab document opens as RGB

- **WHEN** an 8-bit Lab PSD is read
- **THEN** `read_psd` returns a document whose `mode` is `Rgb` and whose composite is the profile-free CIELAB-to-sRGB conversion of the stored planes

#### Scenario: A Bitmap document opens as black and white RGB

- **WHEN** a depth-1 Bitmap PSD is read
- **THEN** `read_psd` returns an RGB document whose pixels are black where the bit is set and white where it is clear

#### Scenario: Grayscale and RGB are unchanged

- **WHEN** an 8-bit Grayscale or RGB PSD is read
- **THEN** `read_psd` returns a document whose mode is `Grayscale` or `Rgb` respectively, exactly as before this change

#### Scenario: Duotone opens as grayscale-normalized RGB

- **WHEN** the header color mode is Duotone (8) at depth 8 with one channel
- **THEN** `read_psd` succeeds, `mode` is `Rgb`, `source_mode` is `Some(Duotone)`,
  and `color_mode_data` is preserved

#### Scenario: One-channel Multichannel opens as grayscale-normalized RGB

- **WHEN** the header color mode is Multichannel (7) at depth 8 with header channel count 1
- **THEN** `read_psd` succeeds and normalizes like grayscale with `source_mode` `Some(Multichannel)`

#### Scenario: Three-channel Multichannel opens as CMY-normalized RGB

- **WHEN** the header color mode is Multichannel (7) at depth 8 with header channel count 3
- **THEN** `read_psd` succeeds and the composite is the profile-free CMY→RGB map of the three plates with `source_mode` `Some(Multichannel)`

#### Scenario: Other Multichannel channel counts stay unsupported

- **WHEN** the header color mode is Multichannel (7) with channel count not in {1, 3}
- **THEN** `read_psd` returns `PsdError::Unsupported` and does not panic

#### Scenario: Unknown mode codes stay unsupported

- **WHEN** the header color mode is any code other than 0,1,2,3,4,7,8,9
- **THEN** `read_psd` returns `PsdError::Unsupported` and does not panic

#### Scenario: Depth 16 and 32 are normalized with the mode

- **WHEN** a 16- or 32-bit CMYK or Lab document is read
- **THEN** the samples are narrowed to 8-bit and the color planes are converted to the working mode per this requirement and `psd-bit-depth`

### Requirement: Layer color planes are converted with the document

When a non-RGB document carries layers, `read_psd` SHALL convert each layer's
color channels with the same conversion as the composite, replacing them with
the working mode's color planes while leaving every non-color channel (`-1`
transparency, `-2` mask, and unmodeled channels) unchanged. A layer whose color
channels do not match the mode's channel layout SHALL be left unchanged rather
than producing a wrong conversion.

#### Scenario: A CMYK layer becomes RGB

- **WHEN** a CMYK document carries a pixel layer with four color channels
- **THEN** the returned layer carries three RGB color channels that are the same profile-aware conversion as the composite, and its transparency channel is unchanged

#### Scenario: Alpha and mask channels are untouched

- **WHEN** a non-RGB layer carries a `-1` transparency channel and a `-2` mask channel
- **THEN** those channels' bytes are the same as in the file

### Requirement: A normalized document records its source mode and saves in it when the mode maps back

When `read_psd` normalizes a non-RGB mode, it SHALL set
`Document.source_mode` to `Some(header_mode)` and SHALL set it to `None` for a
Grayscale or RGB file and for a constructed document. It SHALL convert every
color plane of the document, the composite and each layer, recursing into a
group's descendant layers so a nested pixel layer is converted like a top-level
one. It SHALL drop the Indexed palette from `Document.color_mode_data` on the
normalized document and SHALL instead retain that palette (and each Indexed
index plane, the composite's and every layer's) in the source store, so an
unchanged Indexed document can be written back.

`write_psd` SHALL write a document whose `source_mode` is `Lab` or `Cmyk` back
with that header color mode and, when the read retained native samples, at that
source depth, preferring the source color planes the read retained: it SHALL
re-emit a plane — the composite's or every layer's color channels —
byte-identically at the source depth when the plane is unchanged, and SHALL
otherwise convert it from the working RGB through the document's CMYK profile
(the embedded profile, or the working CMYK profile when the file is untagged)
at 8-bit, widened to the source depth when it is 16/32. The profile-free
`rgb_to_cmyk` inverse SHALL be used only when no CMYK profile is available, and
SHALL remain an exact right-inverse of `cmyk_to_rgb` in that fallback. The Lab
inverse quantizes. A profile-aware CMYK conversion SHALL be within the lcms2
round-trip tolerance and SHALL NOT claim Adobe pixel parity; the profile-free
fallback SHALL be documented as an approximation for an edited plane.

`write_psd` SHALL write an Indexed document back with header color mode Indexed
(one index channel and the retained palette) when its `source_mode` is `Indexed`
and every retained index plane — the composite's when a merged composite is
present, and every pixel layer's — still forward-converts through the palette to
the current working RGB; otherwise it SHALL write the working mode. It SHALL NOT
invent an RGB-to-palette quantization for an edited document.

`write_psd` SHALL write a flat, unchanged Bitmap document back with header color
mode Bitmap (mode 0, depth 1, one 1-bit channel) at its source compression,
re-emitting the retained packed plane byte-identically, when its `source_mode` is
`Bitmap`, it carries no layers or extra channels, and the retained packed plane
still expands to the working RGB. A Bitmap document that has been edited, or that
carries layers or extra channels, SHALL write the working mode; no RGB-to-1-bit
threshold SHALL be invented, and layered or extra-channel Bitmap write-back is
out of scope.

`write_psd` SHALL write an 8-bit Duotone document back with header color mode
8 when its `source_mode` is `Duotone`, it is flat and unedited, and the retained
plane still converts to the working RGB; it SHALL re-emit `color_mode_data`
unchanged. `write_psd` SHALL write an 8-bit Multichannel document back with
header color mode 7 when its `source_mode` is `Multichannel`, its header channel
count was 1 or 3, it is flat and unedited, and the retained plates still convert
to the working RGB. An edited or layered Duotone/Multichannel document SHALL
write the working mode; no plate layout or duotone curve SHALL be invented.

It SHALL re-emit the preserved color-mode-data and image-resource sections. For
a document with no source mode it SHALL continue to write the working mode.

#### Scenario: A Lab document saves as Lab

- **WHEN** an 8-bit Lab document is read and written
- **THEN** the output's header color mode is Lab and an independent decoder's RGB conversion of the output agrees with the input composite within the documented tolerance

#### Scenario: A CMYK document saves as CMYK

- **WHEN** an 8-bit CMYK document is read and written
- **THEN** the output's header color mode is CMYK with four color channels, the unedited color planes are byte-identical to the source, and the output agrees with an independent decoder

#### Scenario: An unedited Lab document saves losslessly

- **WHEN** an 8-bit Lab document is read and written without an edit
- **THEN** the output's Lab planes are byte-identical to the input's and a re-read's working RGB equals the original

#### Scenario: An unedited CMYK document saves losslessly

- **WHEN** an 8-bit CMYK document is read and written without an edit
- **THEN** the output's CMYK color planes are byte-identical to the input's and a re-read agrees with the original working RGB

#### Scenario: An edited Lab plane is re-encoded approximately

- **WHEN** an 8-bit Lab document's composite is edited and written
- **THEN** the changed plane is re-encoded from the edited working RGB with the profile-free inverse, which the spec documents as approximate for saturated colors

#### Scenario: An edited CMYK plane re-encodes through the document profile

- **WHEN** an 8-bit CMYK document with a CMYK profile has a color plane edited and written
- **THEN** the changed plane is encoded through that profile and reading the output back yields the edited RGB within the lcms2 tolerance

#### Scenario: An edited CMYK plane uses the exact inverse

- **WHEN** an untagged CMYK document has a color plane edited and written with no working CMYK profile available
- **THEN** the changed plane uses the profile-free right-inverse and reading the output back yields the edited RGB bytes

#### Scenario: A Lab layer channel is converted

- **WHEN** an 8-bit Lab document with a pixel layer is written
- **THEN** the layer's color channels are Lab-encoded and read back to the same RGB pixels within tolerance

#### Scenario: A CMYK layer channel is converted

- **WHEN** an 8-bit CMYK document with a pixel layer is written
- **THEN** the layer carries four CMYK color channels and reads back to the same RGB pixels within the documented tolerance

#### Scenario: A grouped Lab layer is converted

- **WHEN** an 8-bit Lab document has a pixel layer nested in a group
- **THEN** the nested layer's color channels are converted to RGB on read, like a top-level layer, and the save→read round trip keeps the same pixels

#### Scenario: A 16/32-bit Lab or CMYK source saves back in its source mode and depth

- **WHEN** a 16/32-bit Lab or CMYK document is read and written unchanged
- **THEN** the output's header color mode is Lab or CMYK and its header depth is the source depth, and the retained native color planes are byte-identical to the input's

#### Scenario: A profile-aware CMYK save carries the profile

- **WHEN** an 8-bit CMYK document read through a CMYK profile is written unchanged
- **THEN** the output's image resources still carry that CMYK profile and the retained planes are byte-identical

#### Scenario: Source mode is recorded

- **WHEN** a CMYK or Lab file is read
- **THEN** the document's `mode` is `Rgb` and its `source_mode` is `Some(ColorMode::Cmyk)` or `Some(ColorMode::Lab)`

#### Scenario: Native modes record no source mode

- **WHEN** an RGB or Grayscale file is read, or a document is constructed
- **THEN** its `source_mode` is `None`

#### Scenario: The Indexed palette is consumed

- **WHEN** an Indexed file is read
- **THEN** the returned document's `color_mode_data` is empty

#### Scenario: A flat unchanged Bitmap document saves as Bitmap

- **WHEN** a flat depth-1 Bitmap file with no layers or extra channels is read and written without an edit
- **THEN** the output's header color mode is Bitmap with depth 1 and one color channel, its packed plane is byte-identical to the input's, and a re-read's working RGB equals the original

#### Scenario: An edited or layered Bitmap document saves as the working mode

- **WHEN** a Bitmap document's composite is edited, or the document carries a pixel layer or an extra channel, and it is written
- **THEN** the output's header color mode is RGB and the round-tripped document's `mode` is `Rgb`, because no RGB-to-1-bit threshold is invented and layered/extra-channel Bitmap output is out of scope

#### Scenario: An unchanged Indexed document saves back as Indexed

- **WHEN** an 8-bit Indexed document is read and written without an edit
- **THEN** the output's header color mode is Indexed with one color channel, its palette is byte-identical to the input's, its index planes are byte-identical to the input's, and a re-read's working RGB equals the original

#### Scenario: An edited Indexed document saves as the working mode

- **WHEN** an 8-bit Indexed document's composite or a pixel layer is edited and written
- **THEN** the output's header color mode is RGB and the round-tripped document's `mode` is `Rgb`, because no RGB-to-palette quantization is invented

#### Scenario: An Indexed layer and its group are retained

- **WHEN** an 8-bit Indexed document has a pixel layer nested in a group and is written unchanged
- **THEN** the nested layer's index channel and the composite are re-emitted byte-identically and a re-read matches the original pixels

#### Scenario: An Indexed document without a merged composite saves back as Indexed

- **WHEN** an 8-bit Indexed document with no merged composite and an unchanged pixel layer is written
- **THEN** the output's header color mode is Indexed with one index channel, its palette is byte-identical to the input's, its layer index channel is byte-identical, and no image-data section is written

#### Scenario: An unchanged Duotone document saves as Duotone

- **WHEN** a flat Duotone document is opened and written without edits
- **THEN** the header color mode is 8, `color_mode_data` matches the input, and the plane bytes match the retained source

#### Scenario: An edited Duotone document saves as RGB

- **WHEN** a Duotone document's composite is edited before save
- **THEN** the header color mode is RGB

#### Scenario: An unchanged one-channel Multichannel document saves as Multichannel

- **WHEN** a flat 1-channel Multichannel document is opened and written without edits
- **THEN** the header color mode is 7 and the plate bytes match the retained source

#### Scenario: An unchanged three-channel Multichannel document saves as Multichannel

- **WHEN** a flat 3-channel Multichannel document is opened and written without edits
- **THEN** the header color mode is 7 and the three plate bytes match the retained source

#### Scenario: An edited Multichannel document saves as RGB

- **WHEN** a Multichannel document's composite is edited before save
- **THEN** the header color mode is RGB

## REMOVED Requirements

### Requirement: CMYK and Lab use documented profile-free approximations

**Reason**: CMYK conversions now use the document's embedded CMYK profile or the configured working CMYK profile, so the blanket profile-free requirement is no longer true. Lab remains profile-free and is covered by the replacement requirement.

**Migration**: No document migration. Existing fixtures without an embedded profile keep the profile-free output; tests that assert the integer CMYK formula become fallback tests, and profile-bearing fixtures are added for the lcms2 path.

## ADDED Requirements

### Requirement: CMYK uses the document profile and Lab uses a documented approximation

The CMYK conversion SHALL use the embedded CMYK profile when the document carries one, else the working CMYK profile supplied by the caller, and SHALL use Little CMS 2 through the profile connection space. The profile-free integer formula (`floor(stored_color * stored_black / 255)`, where stored 0 is full ink and 255 is no ink) SHALL remain only as the fallback when no CMYK profile is available, and SHALL be documented as an approximation. The Lab conversion SHALL treat the three planes as `L* = L * 100 / 255`, `a* = a - 128`, and `b* = b - 128` and convert CIELAB under D50 to sRGB through a Bradford D50-to-D65 adaptation; it remains profile-free. Profile-aware CMYK parity is lcms2 parity, not Adobe pixel parity.

#### Scenario: A profile-bearing CMYK pixel uses lcms2

- **WHEN** a CMYK pixel is decoded with an embedded or working CMYK profile
- **THEN** the RGB result equals an independent lcms2 CMYK-to-sRGB conversion within one LSB per channel

#### Scenario: The profile-free fallback is used only without a profile

- **WHEN** a CMYK pixel is decoded with no embedded or working profile
- **THEN** the decoded RGB pixel follows the documented integer formula

#### Scenario: The fallback still decodes black and white

- **WHEN** a profile-free CMYK pixel stores `(0, 0, 0, 0)` or `(255, 255, 255, 255)`
- **THEN** the decoded RGB pixel is `(0, 0, 0)` or `(255, 255, 255)` respectively

#### Scenario: Lab neutral gray and white

- **WHEN** a Lab pixel stores `(200, 128, 128)` or `(255, 128, 128)`
- **THEN** the decoded RGB pixel is a neutral gray near `(194, 194, 194)` or white near `(255, 255, 255)`, within 2 per channel

### Requirement: A profile-bearing CMYK fixture is verified against lcms2

The test corpus SHALL include a CMYK PSD fixture carrying the bundled Photocraft Coated CMYK profile in image resource `1039`, generated by a committed script and stable across runs. A test SHALL compare its decoded composite and a pixel layer against an independent lcms2 CMYK-to-sRGB conversion within one LSB per channel, and SHALL verify that the embedded profile is retained on an unchanged save. The existing profile-free `cmyk.psd` fixture SHALL remain and continue to exercise the fallback.

#### Scenario: The profile-bearing fixture matches lcms2

- **WHEN** the profile-bearing CMYK fixture is read
- **THEN** every composite and layer color channel is within one LSB of an independent lcms2 conversion

#### Scenario: An unchanged profile-bearing save keeps the profile

- **WHEN** the profile-bearing fixture is read and written unchanged
- **THEN** the output carries the same CMYK profile bytes and its planes are byte-identical

#### Scenario: The profile-free fixture still uses the fallback

- **WHEN** the legacy profile-free `cmyk.psd` fixture is read
- **THEN** its decoded pixels equal the documented integer formula
