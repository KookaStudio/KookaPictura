# Asset provenance

Records the origin and license of every asset shipped in the repository, so the
project's provenance posture stays auditable. If you add an asset, add a line
here.

## Vector artwork (`assets/icons`, `assets/cursors`)

- 326 SVG icons and cursors (243 under `assets/icons`, 83 under `assets/cursors`).
- Except where noted below, generated with an AI model (DeepSeek v4.1 Flash)
  and released by the project under **CC0-1.0 / MIT** (reuse under either).
- The `actions.record`, `actions.play`, `actions.stop`, `actions.newSet`,
  `actions.newAction`, `actions.delete`, `actions.modalOn`, and
  `options.airbrush` glyphs are from **lucide-static v1.50.0** (**ISC**) under
  their Lucide slugs (`circle-dot`, `play`, `square`, `folder-plus`,
  `square-plus`, `trash`, `panel-top`, and `spray-can`).
  `actions.modalOff` and `actions.modalPartial` are original project artwork
  based on the Lucide `panel-top` dialog-box shape, released under
  **CC0-1.0 / MIT**.
- `icons/layers.blend-if.svg` is original project artwork drawn by OpenAI Codex as slider handles, released under **CC0-1.0 / MIT**.
- Carry no Adobe/Illustrator/XMP metadata; none are traced from Photoshop.
- `cursors/tool.pen.close.svg` is `tool.pen.svg` with a small circle added by
  hand (the Pen's close-path cursor), under the same licenses.
- `cursors/tool.pathselection.svg` (a black arrow) and
  `cursors/tool.directselection.svg` (a white arrow) are original project
  artwork drawn by hand as plain pointer arrows, under the same licenses.
- `cursors/cursor.pinAdd.svg` (a pushpin with a plus, the Path Blur stop-point
  cursor) is original project artwork drawn by hand in the house cursor
  style, under the same licenses.
- `cursors/cursor.straighten.svg` (a crosshair with a spirit level, the Crop
  tool's straighten-line cursor) is original project artwork drawn by hand in
  the house cursor style, under the same licenses.
- `icons/crop.straighten.svg` (a spirit level, the Crop tool's Straighten
  toggle) is original project artwork drawn by hand in the house icon style,
  released under **CC0-1.0 / MIT**.
- `icons/tool.createplane.svg` (a ruled trapezoid, Vanishing Point's Create
  Plane tool) is original project artwork drawn by hand in the house icon
  style, released under **CC0-1.0 / MIT**.
- The `info.crosshair`, `info.bounds`, and `info.protractor` glyphs are original
  project artwork in the same house style, released under **CC0-1.0 / MIT**.
- `theme.sun.svg` and `theme.moon.svg` are from **lucide-static v1.50.0**
  (**ISC**); `link.github.svg` and `link.discord.svg` are the GitHub and Discord
  marks from **simple-icons v15** (**CC0-1.0**). They are the top-right chrome
  actions: the theme toggle and the GitHub/Discord links.

## UI chrome artwork (`assets/ui`)

- `select-chevron.svg` / `select-chevron-light.svg` are the select drop-down
  chevron (the Lucide "chevron-down" glyph) in each theme family's foreground
  colour; `select-chevron-disabled.svg` is the same glyph in a mid grey for the
  disabled state; `checkbox-check.svg` is the checkbox tick in white. They are
  original project artwork released under **CC0-1.0 / MIT**. The Qt stylesheet
  engine cannot tint an SVG (`currentColor` renders black), so these ship as
  fixed-colour assets; the foreground colours are constant across the four
  brightness levels, so the baked files match a runtime tint.

## Ported vector glyphs (`assets/icons`)

- Eleven panel/path glyphs (`layers.search`, `layers.kindShape`,
  `layers.kindSmartObject`, `layers.reset`, and the seven `path.*` glyphs) are
  extracted from the upstream photorust `LayerIcons.cpp` / `PathIcons.cpp`
  (Source: https://github.com/perfecto25/photorust), relicensed under
  **GPL-3.0-or-later** pending KookaPictura issue #1.

## Ported code

- The shared panel widgets (`AngleDial`, `RampSlider`, `SpectrumBar`,
  `CurveWidget`), the Swatches grid, the Info/Channels/Properties panel
  slices, and the Brightness/Contrast, Vibrance, Exposure, Hue/Saturation,
  Color Balance, Black & White, Photo Filter (its filter colours), Channel
  Mixer, Levels (with
  its `TriangleSlider`), and Curves dialogs are ported from the upstream photorust tree
  (Source: https://github.com/perfecto25/photorust), relicensed under
  **GPL-3.0-or-later** pending KookaPictura issue #1.

## Raster artwork (`assets/icons`)

- `assets/icons/app.png` — the application icon, 512×512 RGBA. Original project
  artwork (the kookaburra mascot), released under **CC0-1.0 / MIT**. Metadata
  stripped, so it carries no Adobe-tool creator strings. Raster only for now;
  a scalable SVG replacement is planned.
- `assets/icons/app.ico` — the same icon in Windows ICO format
  (256/128/64/48/32/16), generated from `app.png` with ImageMagick, released
  under the same **CC0-1.0 / MIT**.
- `assets/icons/app.icns` — the same icon in macOS ICNS format (512/256/128/64/32
  PNG-backed entries), generated from `app.png` with a Pillow-based packer,
  released under the same **CC0-1.0 / MIT**.

## Screenshots

- `assets/screenshot01.jpg` — a capture of the running Kooka Pictura
  application, used in the top-level `README.md`. Original project artwork
  released under **MIT**. Metadata stripped, so it carries no Adobe-tool
  creator strings.

## Fonts

- `crates/pictura-render/assets/LiberationSans-Regular.ttf` — Liberation Sans,
  **SIL Open Font License 1.1**; license text at
  `crates/pictura-render/assets/LICENSES/LiberationSans-OFL.txt`.
- `assets/fonts/NotoSans-{Regular,Medium,SemiBold,Bold}{,Italic,MediumItalic,SemiBoldItalic,BoldItalic}.ttf`
  — Noto Sans (upright and italic at weights 400/500/600/700), vendored from
  `notofonts/noto-fonts` `hinted/ttf/NotoSans/` at commit
  `ffebf8c1ee449e544955a7e813c54f9b73848eac` (the Google Fonts static-instance
  source; Google Fonts distributes the family only as a variable font). The
  application UI font, embedded via `assets/pictura.qrc` and registered at
  startup. **SIL Open Font License 1.1**; license text at
  `LICENSES/NotoSans-OFL.txt`.

## Lens Correction catalogues (`crates/pictura-filters/src/distort/lens_correction/*.tsv`)

Both files are regenerated by `scripts/gen-lens-catalog.py` from the sources
below and hold factual columns only: no images, no calibration coefficients.

- `cameras.tsv` — 3,860 camera bodies: make, model, crop factor, sensor size,
  and a fixed lens's 35 mm range and aperture. From
  `leavestylecode/CameraDatabase` `data/camera_data.csv`
  (https://github.com/leavestylecode/CameraDatabase), **MIT License**,
  Copyright (c) 2025 leavestylecode; license text at
  `LICENSES/CameraDatabase-MIT.txt`. The source repository does not say
  where its specifications came from; its fields match DPReview's
  specification sheets, so only facts (names and measurements) are taken.
- `lenses.tsv` — 1,570 lens entries: make, model name, crop factor, and which
  corrections are calibrated. From the lensfun project's lens list page
  (https://lensfun.github.io/lenslist/). lensfun's database itself is
  **CC BY-SA 3.0**; none of its calibration data is copied, only the factual
  names, crop factors, and calibrated/not flags, credited here to the
  lensfun project.

lensfun's database (CC BY-SA 3.0) is **not bundled**. Lens Correction reads the
copy the system or the user installed (`lensfun-data`, `lensfun-update-data`,
`~/.local/share/lensfun`) at runtime (#932). `profiles.txt` holds one synthetic
profile with invented coefficients, and the lensfun files in tests are
synthetic too.

## Camera colour matrices (`crates/pictura-codec/src/dng/camera_matrices.rs`, `camera_jpeg_matrices.rs`)

Generated by `scripts/gen-camera-matrices.py` (#1465). Each XYZ → camera
matrix is a least-squares **fit** computed from published camera spectral
sensitivities, not a copied table; no Adobe, dcraw/LibRaw (`adobe_coeff`) or
Adobe DNG Converter matrix is used. Inputs, none bundled:

- Camera spectral sensitivities, the 190-patch training reflectances, the
  camera alias list and the CIE 1931 colour-matching functions from
  `AcademySoftwareFoundation/rawtoaces-data`
  (https://github.com/AcademySoftwareFoundation/rawtoaces-data) at commit
  `e9b8503cf8a0641f40e5345d9757bde60c15f423`. The camera, training and alias
  files are **Apache-2.0** (camera files by Weta Digital); the colour-matching
  functions are the CIE's (doi:10.25039/CIE.DS.xvudnb9b, **CC BY 4.0**).
- CIE standard illuminant D65 (doi:10.25039/CIE.DS.hjfjmt59, **CC BY-SA
  4.0**), downloaded at regeneration time; CIE illuminant A is computed from
  its Planckian definition.

The output holds only the fitted numbers and camera names.

The table seeds Canon CR2 bodies (#1465), Canon CR3 bodies (#1474), Nikon NEF bodies (#1469) and Sony
ARW bodies (#1470), every body whose sensitivities rawtoaces-data publishes
for those makes, plus the same-sensor aliases rawtoaces-data's own alias list
names. Three
bodies without published sensitivities reuse a sibling's fitted numbers by
owner decision (#1469): the D100 (D70), D7100 (D5300) and Coolpix P7100
(PowerShot S90); the basis of each is commented in the generated file.

The DC-GX9 is the only Panasonic body rawtoaces-data covers (#1542). No other
openly licensed sensitivities were found for Panasonic bodies: Physlight
(Apache-2.0) has none, ilia3101/SpectralMeasurements carries no licence, and
the Solomatov & Akkaynak estimates are fitted to Adobe DNG Converter output.
By owner decision the other Panasonic rows are therefore **fitted from the
camera's own rendering**: `scripts/fit-camera-jpeg-matrices.py` maps one
sample file's white-balanced camera RGB to the JPEG the camera embedded in
that same file, and builds the matrix pair from the white-balance presets the
file records. The samples (raw.pixls.us files and owner-supplied files) are
not committed; `scripts/camera-jpeg-fits.json` names each one with its
SHA-256 and the fit's error. These rows reproduce Panasonic's colour
rendering, saturation choices included, so they are approximate and not a
colorimetric calibration.

rawtoaces-data covers no Canon CR3 body beyond the eight #1474 added. By the
same owner decision (#1574) the other CR3 rows are fitted from each body's
own camera JPEG (the CR3's full-size JPEG track) and placed under the Daylight,
Cloudy and Tungsten presets of the file's own colour data; the samples are
the raw.pixls.us files `scripts/camera-jpeg-fits.json` names. Canon's preset
layout per colour-data version was located with ExifTool's public tag
documentation and checked on the samples' recorded temperatures. The EOS R1,
R3, R6 Mark III and PowerShot V1 rows (#1579), and the EOS R8 Mark II, EOS M6
Mark II and PowerShot G7 X Mark III rows (#1583, replacing the M6 Mark II's
and G7 X Mark III's same-sensor proxies) are fitted the same way from
owner-supplied files from dpreview's public sample galleries, also not
committed and named with their SHA-256 in `scripts/camera-jpeg-fits.json`.
The PowerShot G1 X Mark III, a CR2 body, is fitted the same way (#1583) from
an owner-supplied CR2: the camera JPEG is IFD0's full-size strip and the
presets come from the Exif maker note's colour data.

Fujifilm RAF files embed no matrix and rawtoaces-data covers no Fujifilm
body. By owner decision (#1473) Fujifilm rows are fitted from each body's own
camera JPEG (the JPEG section the RAF header names) the same way, and placed
under the two white-balance presets the raw directory records (EXIF lights
Standard A and D65), or, for SuperCCD and older EXR bodies without a raw
directory (#1621), the directory records' Fine Weather (`0x2100`) and
Tungsten (`0x2400`) presets; the samples are the raw.pixls.us and
owner-supplied files `scripts/camera-jpeg-fits.json` names, not committed.

## Nikon NEF code tables (`crates/pictura-codec/src/dng/nef_decode.rs`)

The six Huffman code tables of Nikon's compressed NEF (code counts per length
and symbols, for 12- and 14-bit lossy, lossy-after-split and lossless data)
are interoperability constants of the file format: every NEF decoder carries
the same numbers, and no standard publishes them. By owner decision (#1469)
they are taken as format facts; the decoder around them is written in-house
from the format's behaviour and checked against real camera files. No dcraw,
LibRaw or rawspeed code is used.

## Sony ARW coding (`crates/pictura-codec/src/dng/arw_decode.rs`)

The ARW2 block layout, the tone-curve tag and the SR2 private-block cipher (a
key stream seeded by `k × 48828125 + 1` with lagged-XOR feedback) are facts
of the file format, implemented in-house from ExifTool's public
documentation of the Sony tags and observation of real files, by owner
decision (#1470). The decoder was checked by decoding one scene shot
compressed and uncompressed on the same body. No dcraw, LibRaw or rawspeed
code is used.

## Olympus ORF container (`crates/pictura-codec/src/dng/orf.rs`, `orf_meta.rs`)

The headers, the uncompressed packings and the maker-note layouts come from
observation of real camera files (raw.pixls.us, not committed) and ExifTool's
public documentation of the Olympus tags (#1471). Colour uses the matrix each
file embeds, read as camera to sRGB after comparing files with their own
camera JPEGs; it is approximate, and leaves warm tones pink on some early
compacts (SP-350, SP-510UZ, SP-550UZ). No dcraw, LibRaw or rawspeed code is
used in this container reader; the compressed coding below is a separate,
ported module.

## Olympus compressed coding (`crates/pictura-codec/src/dng/orf_decode.rs`)

**Ported code, not an independent implementation.** The AI session that
wrote the ORF container saw a web-search summary quoting fragments of
third-party decoders of this coding, and a separate agent's attempt to
derive it from patents and samples stopped on recognising training memory
of such a decoder; nothing from either was kept. By owner decision (#1535)
the coding is then ported openly from LibRaw 0.22.1 (Source:
https://github.com/LibRaw/LibRaw, tag `0.22.1`, commit
b860248a89d9082b8e0a1e202e516f46af9adb29, file
`src/decoders/olympus14.cpp`): the stream header check, the bit pump, the
adaptive code with its escape, the fine context and the neighbour
predictor. LibRaw is dual-licensed **LGPL-2.1** or **CDDL-1.0**; the port
takes the LGPL-2.1 and is distributed under **GPL-3.0-or-later** as
LGPL-2.1 section 3 permits. The file carries LibRaw's copyright notice
(Alex Tutubalin, LibRaw LLC). The port was checked sample for sample
against LibRaw 0.22.1's raw plane (through rawpy) on 23 compressed files
from 17 bodies, E-410 to OM-1 Mark II, two of them 14-bit (raw.pixls.us and
local samples, not committed).

Patents (OVR-004): a Google Patents status search on 2026-11-22 listed
US7715639B2 as active, with an adjusted expiration of 2029-02-01, and
US7333035B2, US7259698B2 and JP2006211155A (JP4434974B2) as expired.
Whether the active patent covers ORF files or decoding them has not been
assessed; this is not a legal conclusion, and the owner decides before
merge.

## Panasonic RW2 container (`crates/pictura-codec/src/dng/rw2.rs`)

The header, tags and three plain packings (16-bit words, RawFormat 2 and 7)
come from observation of real camera files (raw.pixls.us, not committed) and
ExifTool's public documentation of the Panasonic tags (#1472). The packings
were found by trying plain byte and bit orders and keeping the smoothest
image. No dcraw, LibRaw or rawspeed code is used, and no colour matrix is
included.

The #1472 research also printed the header of an Adobe-converted DNG from
the sample index, with truncated matrix values; the file was deleted and
nothing from it is used.

## Panasonic RW2 compressed codings (`crates/pictura-codec/src/dng/rw2_decode.rs`)

**Ported code, not an independent implementation.** The AI model that wrote
the container has training exposure to open-source decoders of these codings,
so by owner decision (#1541) they are ported openly from darktable's rawspeed
(Source: https://github.com/darktable-org/rawspeed, commit
718eae230df9ebb1750755b775e52185f2342a46): the `PanasonicV4`, `V5`, `V6` and
`V8` decompressors, `Rw2Decoder`'s coding choice and black-level offset, and
`RawImageDataU16::fixBadPixel`. rawspeed is **LGPL-2.1-or-later**; the port is
distributed under **GPL-3.0-or-later** as LGPL-2.1 section 3 permits, and the
file carries rawspeed's copyright notices (Klaus Post, Pedro Côrte-Real, Axel
Waggershauser, Roman Lebedev, Alexey Danilchenko, Stefan Hoffmeister, LibRaw
LLC, Daniel Vogelbacher, Kolton Yager). The port was checked against LibRaw
0.22.1's unpacked sensor data on sample files (raw.pixls.us, not committed).

Patents (OVR-004): a Google Patents status search on 2026-11-22 found active
Panasonic-origin patents on fixed-length predictive RAW coding (US8090209B2,
US9106250B2, now assigned to Nuvoton Technology Corp Japan). Whether any
claim reads on these codings or on decoding them has not been assessed; this
is not a legal conclusion.

## Canon CRX coding (`crates/pictura-codec/src/dng/crx.rs`, `crx_lines.rs`, `crx_wavelet.rs`)

**Ported code, not an independent implementation.** Every CR3 frame is
CRX-coded, and the AI model that wrote the container has training exposure
to LibRaw's open-source CRX decoder, so by owner decision (#1474) the coding
is ported openly from LibRaw 0.22.1 (Source:
https://github.com/LibRaw/LibRaw, tag `0.22.1`, commit
b860248a89d9082b8e0a1e202e516f46af9adb29, file `src/decoders/crx.cpp`):
the bit reader, the Golomb-Rice line decoders, inverse quantization, the
inverse 5/3 wavelet, the tile, plane and subband headers, the quantizer
tables and encoding 3's plane combination. LibRaw is dual-licensed
**LGPL-2.1** or **CDDL-1.0**; the port takes the LGPL-2.1 and is distributed
under **GPL-3.0-or-later** as LGPL-2.1 section 3 permits. Each file carries
LibRaw's copyright notices (Alexey Danilchenko; Alex Tutubalin, LibRaw LLC).
The port was checked bit-exact against LibRaw 0.22.1's unpacked sensor data
(through rawpy) on 52 sample files from 29 bodies (raw.pixls.us, not
committed).

## Canon CR3 container (`crates/pictura-codec/src/dng/cr3.rs`)

The box walk follows ISO/IEC 14496-12 and public write-ups of the CR3
layout; the raw-track choice, the `CMP1` fields and the `CTMD` record walk
follow LibRaw 0.22.1's `src/metadata/cr3_parser.cpp` (Copyright (C)
2019-2025 LibRaw LLC, LGPL-2.1). The maker-note readers are the CR2
reader's. No colour matrix comes from the file.

Patents (OVR-004): a Google Patents status search on 2026-11-23 found active
Canon Inc patents on related coding: US11140392B2 (Bayer RAW split into
tiles and planes, sub-band transform, quantization parameters shared across
sub-bands per segment and applied across tile boundaries; priority
2018-06-15, adjusted expiry 2039-06-26) and US10609393B2 (raster-order
coding switching between run-length and Golomb-Rice prediction coding, for
colour planes or wavelet sub-bands; priority 2016-04-14, anticipated expiry
2037-04-10). Google lists both as active, which is its assumption. Whether
any claim reads on the CRX coding or on decoding it has not been assessed;
this is not a legal conclusion.

## Test fixtures

- `crates/pictura-codec/tests/fixtures/**` — generated by
  `scripts/generate-fixtures.py` (independent `psd-tools`), except the two
  smart-object fixtures below.
- `assets/test_with_smart_object0{1,2}.psd` — generated by
  `cargo run -p pictura-codec --example make_smart_fixtures` with the project's
  own writer. Synthetic; contain no Adobe-authored bytes.
- `crates/pictura-codec/tests/fixtures/engine_data.bin` — synthetic EngineData
  authored via `scripts/generate-fixtures.py`.
- `crates/pictura-codec/tests/fixtures/pict_rgb.pct` and `pict_jpeg.pct` —
  generated by ImageMagick from a deterministic 4×3 PPM written for the PICT
  tests. The pixel content is original project content under **CC0-1.0 / MIT**;
  the PICT container bytes are ImageMagick's output (ImageMagick License,
  Apache-2.0-compatible). No Adobe tool or file was used.

## Colour profiles (`assets/icc`)

- `icc/photocraft-coated-cmyk.icc` (236,844 bytes, SHA-256
  `f7ba932085778fac4f8fed66e0a048351b44bb827eb07c1360b570bd3bfa54cc`):
  *Photocraft Coated CMYK (synthetic, 300% TAC, medium GCR)*, an ICC v4 output
  (`prtr`) CMYK profile copied verbatim from the PhotoCraft project
  (`crates/cms/profiles/`, PhotoCraft contributors). It is generated by
  PhotoCraft's own Yule–Nielsen modified Neugebauer model from published
  ISO 12647-2 coated-paper aims, not measured and not derived from any Adobe or
  vendor profile. Its copyright tag dedicates it to the public domain
  (**CC0-1.0**). Used as the PDF CMYK destination and PDF/X output intent
  (#626); a plausible coated-offset profile, not a press characterisation.

## Not distributed

- `reference-fixtures/` is gitignored. It holds real Photoshop-authored files
  kept only for optional local reference tests; they are never committed or
  shipped.

## Fujifilm RAF container (`crates/pictura-codec/src/dng/raf.rs`)

The header, the directory records, the raw directory and the three
uncompressed codings come from observation of real camera files
(raw.pixls.us, not committed) and ExifTool's public documentation of the RAF
tags (#1473). The packed bit orders were found by trying candidate orders and
keeping the smoothest image; the X-Trans phase was checked against the DNG
Adobe's converter wrote from the same raw.pixls.us X100S file (its CFA
pattern and samples only, compared in a test), and the Bayer phase by
correlating with the camera JPEG. No dcraw, LibRaw or rawspeed code is used
in this container reader; the compressed coding and the SuperCCD geometry
below are separate, ported modules.

For the older sensor sections of plain words (#1621) the reader follows
LibRaw 0.22.1's observable behaviour, checked word for word against its raw
plane (through rawpy) on 19 sample files: words in the camera JPEG's Exif
byte order, both SuperCCD SR frames side by side when the layout's top bit
is set, black from record `0x4000` per colour and green row, LibRaw's white
`0x3E00` (4095 for big-endian words). The 2 × 2 layout record's colour
decode (bytes last first, low two bits the colour) is ours, confirmed
against LibRaw's raw pattern on every upright sample, as is `0xF006`
selecting most-significant-bit-first 12-bit packing (the S1 and SL1000).

The SuperCCD SR merge (#1648, `crates/pictura-codec/src/dng/raf_sr.rs`) is
our own design. Where the R frame sits (header `0x80`, LibRaw's second-shot
offset) and LibRaw's guard on `0x78` follow LibRaw 0.22.1's observable
behaviour; the ratio fit, blend and headroom are not taken from any
converter (Fujifilm's and Adobe's merges are closed, and LibRaw reads one
frame at a time). Record `0x9200`'s nominal ratio of 16 was read from the
S3 Pro, S5 Pro and F700 sample files.

## Fujifilm SuperCCD geometry (`crates/pictura-codec/src/dng/raf_superccd.rs`, `crates/pictura-codec/src/dng/demosaic/superccd.rs`)

**Ported code, not an independent implementation.** By owner decision
(#1621) the SuperCCD geometry is ported openly from LibRaw 0.22.1 (Source:
https://github.com/LibRaw/LibRaw, tag `0.22.1`, commit
b860248a89d9082b8e0a1e202e516f46af9adb29): the `0x0130` layout bits and
`0x0121` size of `parse_fuji` (`src/metadata/fuji.cpp`), the active area
and `fuji_width` grid of `identify` (`src/metadata/identify.cpp`), the
repack of `raw2image` (`src/preprocessing/raw2image.cpp`) and the rotation
of `fuji_rotate` (`src/postprocessing/aspect_ratio.cpp`). LibRaw is
dual-licensed **LGPL-2.1** or **CDDL-1.0**; the port takes the LGPL-2.1 and
is distributed under **GPL-3.0-or-later** as LGPL-2.1 section 3 permits. The
files carry LibRaw's copyright notice (LibRaw LLC; dcraw.c, Dave Coffin).
The grid was checked against LibRaw's own repack of its raw plane within one
level on 10 SuperCCD files, and the upright rendering against LibRaw's
(block means correlating at 0.9995 or more in each channel), through rawpy
(raw.pixls.us and owner files, not committed).

## Fujifilm compressed coding (`crates/pictura-codec/src/dng/raf_compressed.rs`)

**Ported code, not an independent implementation.** By owner decision
(#1620) Fujifilm's compressed RAF coding is ported openly from LibRaw 0.22.1
(Source: https://github.com/LibRaw/LibRaw, tag `0.22.1`, commit
b860248a89d9082b8e0a1e202e516f46af9adb29, file
`src/decoders/fuji_compressed.cpp`): the header checks, the strip and
quantizer-base layout, the quantizer tables, the gradient-adaptive codes,
the six-pass X-Trans and Bayer line decode and the copy through the CFA.
LibRaw is dual-licensed **LGPL-2.1** or **CDDL-1.0**; the port takes the
LGPL-2.1 and is distributed under **GPL-3.0-or-later** as LGPL-2.1 section
3 permits. The file carries LibRaw's copyright notices (Alexey Danilchenko;
Alex Tutubalin, LibRaw LLC). The port was checked sample for sample against
LibRaw 0.22.1's raw plane (through rawpy) on 13 compressed files from 8
bodies, lossless and lossy, X-Trans and Bayer, 14 and 16 bits (raw.pixls.us,
not committed).

Patents (OVR-004): a Google Patents status search on 2026-11-26 for
Fujifilm-assigned patents on lossless or lossy compression of RAW or
colour-filter-array data with gradient-context adaptive coding found none;
the hits were other assignees' general adaptive Golomb-Rice and CFA
compression patents. A search that finds nothing is not clearance, and this
is not a legal conclusion.

The X-Trans demosaic (`crates/pictura-codec/src/dng/demosaic/xtrans.rs`) is
designed here; it does not follow a published or third-party method.
