# Asset provenance

Records the origin and license of every asset shipped in the repository, so the
project's provenance posture stays auditable. If you add an asset, add a line
here.

## Vector artwork (`assets/icons`, `assets/cursors`)

- 238 SVG icons and cursors.
- 252 SVG icons and cursors.
- Generated with an AI model (DeepSeek v4.1 Flash) and released by the project
  under **CC0-1.0 / MIT** (reuse under either).
- Carry no Adobe/Illustrator/XMP metadata; none are traced from Photoshop or a
  third-party icon pack.
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

## Test fixtures

- `crates/pictura-codec/tests/fixtures/**` — generated by
  `scripts/generate-fixtures.py` (independent `psd-tools`), except the two
  smart-object fixtures below.
- `assets/test_with_smart_object0{1,2}.psd` — generated by
  `cargo run -p pictura-codec --example make_smart_fixtures` with the project's
  own writer. Synthetic; contain no Adobe-authored bytes.
- `crates/pictura-codec/tests/fixtures/engine_data.bin` — synthetic EngineData
  authored via `scripts/generate-fixtures.py`.

## Not distributed

- `reference-fixtures/` is gitignored. It holds real Photoshop-authored files
  kept only for optional local reference tests; they are never committed or
  shipped.
