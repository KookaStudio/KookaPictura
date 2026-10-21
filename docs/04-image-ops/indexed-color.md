# Indexed Color

- **Spec ID:** `IMG-008`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Indexed Color dialog, Color Table command, and palette options are carried forward from CS5 unchanged in the CS6 Help.
- **Depends on:** `04-image-ops/image-modes.md`, `04-image-ops/bit-depth-and-conversion.md`, `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/file-formats.md` (`ARCH-011`), `10-workflow-io/export-formats.md`, `10-workflow-io/web-export-and-slices.md`, `07-color-painting/color-picker.md`.

> All module, crate, and widget names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Statements marked *(inferred)* are not taken from a
> fetched source and are candidates for `## Open questions`.

## CS6 behavior

Indexed Color mode is `Image > Mode > Indexed Color`. It produces 8-bit images
with **up to 256 colors** via a **color lookup table (CLUT)**. Source: CS6
reference, "Indexed Color mode", "Convert a grayscale or RGB image to indexed
color", "Conversion options for indexed-color images", "Customize indexed color
tables", "Color Table".

- The source must be **8 bits/channel Grayscale or RGB**. For grayscale the
  conversion is automatic; for RGB the Indexed Color dialog appears.
- All **visible layers are flattened**; **hidden layers are discarded**.
- If a color is not in the table, Photoshop picks the closest color or
  **dithers** to simulate it.
- Indexed color reduces file size and suits multimedia/web use; editing is
  limited (convert back to RGB for extensive work).
- Indexed files can be saved to Photoshop, BMP, DICOM, GIF, Photoshop EPS,
  PSB, PCX, Photoshop PDF, Photoshop Raw, Photoshop 2.0, PICT, PNG, Targa, TIFF.
- Indexed Color and 32-bit images **cannot** be converted to Multichannel mode.

### Indexed Color dialog (RGB source)

- `Preview`.
- **Palette type**: `Exact` (only if the image uses ≤256 colors, so no
  dithering), `System (Mac OS)`, `System (Windows)`, `Web` (216 colors), 
  `Uniform` (uniform RGB-cube sampling; the count is the nearest perfect cube —
  8, 27, 64, 125, 216 — ≤ the Colors value), and local/master perceptual
  families:
  - `Local (Perceptual)`, `Local (Selective)`, `Local (Adaptive)` — palette
    based on the current image's colors.
  - `Master (Perceptual)`, `Master (Selective)`, `Master (Adaptive)` — same but
    taking **all open documents** into account.
  - `Custom` — opens the Color Table dialog; also shows the current adaptive
    palette.
  - `Previous` — reuses the palette from the previous conversion.
- **Colors**: exact number of colors (up to 256) for Uniform, Perceptual,
  Selective, Adaptive. The box controls palette creation only; the image is
  still treated as an 8-bit 256-color image.
- **Forced**: `Black And White` (pure black + white), `Primaries` (R/G/B/C/M/Y/K
  + white), `Web` (216 web-safe), `Custom` (user-defined).
- **Transparency**: preserve transparent areas by adding a special table entry;
  if off, transparent areas are filled with the matte or white.
- **Matte**: background color for anti-aliased edges adjacent to transparency;
  options include None / foreground / background / white / a chosen color
  *(exact list inferred)*. `None` gives hard-edged transparency (or fills
  transparent areas with 100% white when Transparency is off). Matte is only
  available when the image has transparency.
- **Dithering** (unless `Exact`): `None` (nearest color, posterized),
  `Diffusion` (error diffusion; `Preserve Exact Colors` protects table colors),
  `Pattern` (halftone-like square pattern), `Noise` (reduces seam patterns at
  slice edges). A dither **amount** percentage is also entered; a higher amount
  dithers more colors and may increase file size.

### Color Table command

`Image > Mode > Color Table` (also from the Indexed Color dialog via
`Custom`) edits the table:

- Click a single color to change it, or drag to select a range and set first/last
  colors (the range is filled by interpolation).
- Select the Eyedropper and click a color in the table or image to assign
  **transparency** to that color.
- Predefined tables (`Table` menu): `Custom`, `Black Body` (black→red→orange→
  yellow→white), `Grayscale` (256 grays), `Spectrum` (violet→blue→green→yellow→
  orange→red), `System (Mac OS)`, `System (Windows)`.
- `Save` / `Load` color tables for reuse with other images (and with the Swatches
  panel).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Mode > Indexed Color` | Menu + dialog | none documented | Flattens visible layers, discards hidden. |
| Indexed Color dialog | Dialog | — | Palette, Colors, Forced, Transparency, Matte, Dithering, Preview. |
| `Image > Mode > Color Table` | Menu + dialog | none documented | Also reached from the Indexed Color dialog's `Custom`. |
| Color Table dialog | Dialog | — | Edit colors, assign transparency via eyedropper, predefined tables, Save/Load. |
| `File > Save As` | Dialog | — | GIF/PNG-8/BMP/… indexed outputs. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Palette | combo | `Local (Selective)` *(inferred)* | Exact, System (Mac/Windows), Web, Uniform, Local/Master (Perceptual/Selective/Adaptive), Custom, Previous | Exact only when ≤256 colors. |
| Colors | int | 256 | 1–256 | Applies to Uniform/Perceptual/Selective/Adaptive. |
| Forced | combo | None *(inferred)* | None / Black And White / Primaries / Web / Custom | Forces entries into the table. |
| Transparency | bool | on *(inferred)* | on / off | Adds a transparent index. |
| Matte | combo | None / white *(inferred)* | None / foreground / background / white / custom | Available only with transparency. |
| Dither | combo | Diffusion *(inferred)* | None / Diffusion / Pattern / Noise | Not shown for Exact. |
| Dither amount | percent | 75% *(inferred)* | 0–100% *(inferred)* | Higher = more dithering. |
| Preserve Exact Colors | bool | on *(inferred)* | on / off | Diffusion only. |
| Color table entry | color | — | Color Picker / Editor | Per-table. |
| Transparency index | color/eyedropper | — | one entry | Assigned in the Color Table dialog. |

Defaults beyond the Help's explicit statements are *(inferred)* and listed in
`## Open questions`.

## Algorithms & pipeline

1. **Quantisation** (`Palette`): produce ≤ `Colors` representative colors.
   - `Exact`: the unique colors of the image (always ≤256 here).
   - `System`/`Web`: fixed palettes (Mac/Windows 256; 216 web-safe).
   - `Uniform`: sample the RGB cube uniformly (k levels per axis; 6³ = 216).
   - `Perceptual`: prioritise colors the eye is more sensitive to.
   - `Selective`: like Perceptual but favour broad areas and preserve web colors
     (usually best integrity).
   - `Adaptive`: the most frequently occurring colors; can be weighted by a
     selection.
   - `Custom`/`Previous`: supplied table.
   The Classic quantisation algorithms are **median cut** (Heckbert 1982) and
   **octree**, with k-means variants; Adobe's exact method is unpublished:
   `Perceptual`/`Selective`/`Adaptive` are **behavioural parity only, algorithm
   TBD**.
2. **Forced colors** are inserted into the table before/again after
   quantisation, shrinking the available quantised slots.
3. **Mapping**: each pixel maps to the nearest palette entry under the working
   color-difference metric, or is dithered:
   - `None`: nearest entry (posterized).
   - `Pattern`: ordered/halftone threshold matrix.
   - `Diffusion`: error diffusion (Floyd–Steinberg-class).
   - `Noise`: randomised threshold.
4. **Transparency**: reserve one index for transparency (PSD resource 1047
   records the index); the matte color fills anti-aliased edges.
5. **Color Table edit**: direct LUT editing and interpolation of ranges.

## Rust module mapping

Proposed:

- `pictura-core::color::indexed` —
  `PaletteType { Exact, SystemMac, SystemWindows, Web, Uniform, Local(Perceptual|Selective|Adaptive), Master(...), Custom, Previous }`,
  `DitherMethod { None, Diffusion { preserve_exact: bool }, Pattern, Noise }`,
  `IndexedConversionOptions { palette, colors: u16, forced: ForcedColors, transparency: bool, matte: Option<Color>, dither, dither_amount: f32 }`.
- `Palette { entries: Vec<Rgb8>, transparent_index: Option<u8> }`.
- `fn quantize(img: &PixelBuffer, opts) -> Palette`.
- `fn to_indexed(img, opts) -> (IndexedBuffer, Palette)`; `fn to_rgb(buf, palette) -> PixelBuffer`.
- `fn edit_color_table(palette, edit: TableEdit)`.
- `IndexedConvertCommand` implements `EditCommand`.

## Qt6 component mapping

- `IndexedColorDialog` (`QDialog`) — Palette/Colors/Forced/Transparency/Matte/
  Dither combos + spin boxes, dither amount, `Preview`.
- `ColorTableDialog` — scrollable grid of swatches, eyedropper transparency
  tool, `Table` menu, `Save`/`Load`.
- A shared `PaletteGridWidget` reused by `07-color-painting/swatches-and-libraries.md`.

## Data-model impact

- Document color mode = **Indexed (mode 2)**, depth 8; pixels are palette
  indices, not colors. The model needs an `IndexedBuffer` + `Palette` rather than
  an RGB buffer.
- PSD **Color Mode Data Section**: indexed images store a **768-byte** color
  table (non-interleaved order). PSD image resource **1046** = Indexed Color
  Table Count (number of actually defined colors); **1047** = Transparency Index.
- Conversion to Indexed is **lossy and flattening**: undo stores the
  pre-conversion pixels and layer structure (`ARCH-009`).
- The `Palette` should serialise to/from the PSD 768-byte table and a `.act`/
  table file for `Save`/`Load` (file format `ARCH-011`).

## Edge cases

- **Source bit depth**: must be 8-bit Grayscale/RGB; 16/32-bit must convert to
  8-bit first.
- **>256 colors**: quantisation required (Exact unavailable).
- **Hidden layers**: discarded; visible flattened — data loss the user must be
  warned about.
- **Transparency + Matte**: with Transparency off, transparent areas fill matte
  or white; `None` matte yields hard edges.
- **Forced colors**: forcing many colors can exceed 256 after quantisation — the
  table must be capped and forced entries prioritised.
- **Exact + no dither**: the Help states Exact implies no dithering.
- **Uniform count**: result snaps to the nearest perfect cube ≤ Colors.
- **Local vs Master**: Master consults all open documents; Local only the
  current image.
- **Previous** reuse across documents.
- **Color Table editing** can make the image look wrong if the loaded table's
  positions differ; the Help notes colors change to the referenced positions.
- **Multichannel**: conversion forbidden from Indexed (and 32-bit).
- **Huge/PSB**: quantisation must stream or subsample; avoid a full float palette
  buffer.

## Parity acceptance criteria

1. Given an RGB image with ≤256 unique colors and Palette = Exact, the result
   contains every original color and no dithering occurs; converting back to RGB
   is byte-identical.
2. Given Uniform with Colors = 216, exactly 216 entries are produced
   (6×6×6); with Colors = 200 the count snaps to 125.
3. Given Web, every palette entry is in the 216 web-safe set.
4. Given Transparency on, PSD resource 1047 records a valid transparent index and
   transparent pixels map to it; with Transparency off they map to the matte or
   white.
5. Given Dither = Diffusion with Preserve Exact Colors on, pixels whose colors
   exist in the table are not dithered.
6. Given a PSD with an indexed image, saving writes a 768-byte Color Mode Data
   table and resource 1046/1047; reopening reproduces the same palette and pixel
   indices.
7. Given the Color Table dialog and an eyedropper on an image color, that index
   becomes the transparent entry.
8. Given a 16-bit document, `Image > Mode > Indexed Color` is unavailable until
   converted to 8-bit.
9. Given hidden layers, converting to Indexed discards them and the user is
   warned (CS6 behaviour: silent discard — parity requires the same visible
   result).

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help PDF, "Indexed Color mode" (8-bit, 256 colors, CLUT, closest
  color/dither, file formats); "Convert a grayscale or RGB image to indexed
  color" (8-bit source, flatten/discard, dialog); "Conversion options for
  indexed-color images" (Palette types incl. Local/Master/Previous, Colors,
  Forced, Transparency, Matte, Dithering + amount); "Customizing indexed color
  tables" / "Color Table" (edit, transparency via eyedropper, predefined tables,
  Save/Load); "Multichannel mode" note that Indexed/32-bit cannot convert.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD spec: mode enum `Indexed = 2`; Color Mode Data Section for indexed =
  768-byte table in non-interleaved order; resources 1046 (Indexed Color Table
  Count) and 1047 (Transparency Index); color-space enum includes Duotone.
- `https://multimediaconstruction.com/Color-Reduction/reduce-sec2.php` —
  community description of Selective/Adaptive/Perceptual palettes. Secondary.
- `https://en.wikipedia.org/wiki/Color_quantization` and
  `https://en.wikipedia.org/wiki/Median_cut` — standard quantisation algorithms
  (median cut, octree, k-means) referenced as behavioural models. Secondary.

## Open questions

- **Exact quantisation algorithm** for Perceptual/Selective/Adaptive in CS6
  (median cut? octree? proprietary?). *Resolves with:* palette comparisons
  against reference implementations, or an Adobe statement.
- **Default palette type / Colors / Forced / Dither / dither amount / Matte** in
  a fresh dialog. *Resolves with:* CS6 screenshots.
- **Color-difference metric** used for nearest-color mapping and dithering
  (RGB Euclidean? Lab?). *Resolves with:* CS6 pixel tests on a controlled chart.
- **Matte option list** in CS6. *Resolves with:* CS6 dialog capture.
- **Diffusion kernel** (Floyd–Steinberg vs a variant) and `Pattern` matrix.
  *Resolves with:* dither pattern analysis.
- **Behavior with >256 forced colors.** *Resolves with:* CS6 stress test.
- **Palette file format** for Color Table `Save`/`Load` and Swatches interaction.
  *Resolves with:* inspecting CS6 `.act`/table files.
- **Whether `Local`/`Master` use identical quantisation** except for the sample
  set. *Resolves with:* CS6 tests.
