# Asset provenance

Records the origin and license of every asset shipped in the repository, so the
project's provenance posture stays auditable. If you add an asset, add a line
here.

## Vector artwork (`assets/icons`, `assets/cursors`)

- 325 SVG icons and cursors (242 under `assets/icons`, 83 under `assets/cursors`).
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

## Camera colour matrices (`crates/pictura-codec/src/dng/camera_matrices.rs`)

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

The table seeds Canon CR2 bodies (#1465), Nikon NEF bodies (#1469) and Sony
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
used.

The compressed coding is not implemented (#1535). The AI session that wrote
this container saw a web-search summary quoting fragments of third-party
decoders of that coding, and a separate agent's attempt to derive it stopped
on recognising training memory of such a decoder. Nothing from either is
included.

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

## Test fixtures

- `crates/pictura-codec/tests/fixtures/**` — generated by
  `scripts/generate-fixtures.py` (independent `psd-tools`), except the two
  smart-object fixtures below.
- `assets/test_with_smart_object0{1,2}.psd` — generated by
  `cargo run -p pictura-codec --example make_smart_fixtures` with the project's
  own writer. Synthetic; contain no Adobe-authored bytes.
- `crates/pictura-codec/tests/fixtures/engine_data.bin` — synthetic EngineData
  authored via `scripts/generate-fixtures.py`.

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
