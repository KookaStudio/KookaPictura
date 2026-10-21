# Web Export and Slices

- **Spec ID:** `WF-005`
- **Status:** `Draft`
- **Parity tier:** `Core` (Save for Web & Devices for Photoshop: GIF/JPEG/PNG-8/PNG-24/WBMP, slices, HTML/CSS output, Image Size); `Extended-only`/`Non-goal (Linux)` (Device Central integration; SWF/SVG optimization are Illustrator-only and not Photoshop)
- **New in CS6:** `Changed` — the dialog is named **Save For Web & Devices** and the still-image optimization matrix is the CS5 set. CS6 adds the dark interface and Copy CSS from layers (a **Creative Cloud** feature shown in the Help corpus, **not** CS6) — see `## Open questions`. `Copy CSS from layers` must not be offered as CS6 parity.
- **Depends on:** `ARCH-011` file-formats, `ARCH-008` document-model, `ARCH-007` color-management, `WF-003` save-and-save-as, `WF-004` export-formats, `03-tools/slice-tools.md`, `02-ui-ux/panels/layers-panel.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. CS6 behavior is taken from the fetched CS6 Help PDF
> unless marked *(inferred)*.

## CS6 behavior

### Save for Web & Devices

`File > Save For Web & Devices` opens the optimization dialog. It previews
optimized images in different formats and attributes, lets you compare multiple
versions simultaneously, and can generate an HTML file alongside the image(s).

Display options (tabs above the image area):

- **Original** — no optimization.
- **Optimized** — current optimization settings applied.
- **2-Up** — two versions side by side.
- **4-Up** — four versions side by side; pick a pane to optimize it. In 4-Up,
  **Repopulate Views** (Optimize menu) regenerates lower-quality variants after a
  settings change.

**Navigating**: Hand tool (or hold spacebar) to pan; Zoom tool to zoom; typed or
bottom-of-dialog magnification.

**Annotations**: below each pane, the original shows file name and size; the
optimized shows the current options, optimized file size, and estimated download
time using the selected **modem speed** (chosen in the Preview pop-up menu).

**Gamma preview (Preview pop-up menu)**: **Monitor Color** (default, no
adjustment), **Legacy Macintosh (No Color Management)** (gamma 1.8), **Windows
(No Color Management)** (gamma 2.2), **Use Document Profile**. Preview-only; does
not affect output.

**Workflow**:

1. Choose `File > Save For Web & Devices`.
2. Pick Optimized / 2-Up / 4-Up; in 4-Up select the pane.
3. If the image has multiple slices, select the slices to optimize.
4. Choose a **Preset** or set individual options (options depend on format).
5. Fine-tune quality vs. size; optimize every slice.
6. If the document has a non-sRGB embedded profile, **Convert to sRGB** (default
   on) before saving so browsers match.
7. **Metadata** menu chooses what metadata to save.
8. **Save**.
9. **Save Optimized As** dialog: file name, destination, **Format** (HTML and
   Images / Images Only / HTML Only), optional output settings, and (for slices)
   **Slices** = All Slices or Selected Slices. **Alt/Option + Reset** resets to the
   last saved optimization settings; **Alt/Option + Remember** keeps them for next
   time.

### Web graphic formats

Bitmap (raster), resolution-dependent: **GIF, JPEG, PNG (PNG-8/PNG-24), WBMP**.
Vector: **SVG and SWF** — Illustrator-only in the Save For Web dialog, so **not
Photoshop**. Photoshop does not offer SVG/SWF optimization here.

### Format option matrix

| Format | Options |
|---|---|
| **JPEG** | Quality; **Optimized**; **Progressive** (requires Optimized); **Blur**; **Embed Color Profile**; **Matte** |
| **GIF** | **Lossy**; Color Reduction Method + Colors; Dithering Method + Dither %; Transparency + Matte; Transparency Dithering; **Interlace**; **Web Snap**; color table |
| **PNG-8** | Same as GIF except **no Lossy** |
| **PNG-24** | Transparency (up to 256 levels); **Interlace** |
| **WBMP** | Dithering algorithm (No Dither / Diffusion / Pattern / Noise) + amount |

Details:

- **JPEG Quality**: higher preserves more detail and increases size. **Optimized**
  yields a slightly smaller file (older browsers may not support it). **Progressive**
  displays increasingly detailed versions during download (requires Optimized; more
  RAM; some browsers unsupported). **Blur** applies a Gaussian-Blur-like effect to
  improve compression; recommended 0.1–0.5. **Embed Color Profile** preserves the
  profile. **Matte** fills/blends pixels that were transparent.
- **GIF Lossy**: selectively discards data; values of 5–10, sometimes up to 50, are
  usable; can reduce size 5–40%. Incompatible with Interlace and with Noise/Pattern
  dither.
- **Color Reduction Method**: **Perceptual**, **Selective** (default, favors broad
  color areas and web colors), **Adaptive**, **Restrictive (Web)** (216-color
  web-safe), **Custom**, and fixed palettes **Black and White, Grayscale, Mac OS,
  Windows**. **Colors** sets the lookup-table size (2–256).
- **Dithering Method**: **Diffusion** (random, diffused), **Pattern**
  (halftone-like square pattern), **Noise** (random, not diffused, no seams); plus
  a **Dither** percentage.
- **Transparency and Matte** (GIF/PNG-8): with **Transparency** on, fully
  transparent pixels stay transparent and partially transparent pixels blend with
  the **Matte** color; with Transparency off, transparent pixels are filled with
  the matte and partially transparent pixels blend to it. Selecting Transparency
  with **None** matte makes >50%-transparent pixels fully transparent and ≤50%
  fully opaque.
- **Transparency Dithering** (when Transparency on): **No Transparency Dither**,
  **Diffusion**, **Pattern**, **Noise**.
- **PNG-24** supports multilevel (up to 256-level) transparency; the **Matte**
  option is disabled because it can blend with any background.
- **Interlace** (GIF/PNG) shows a low-resolution version while downloading; larger
  file.
- **Web Snap** shifts colors toward the closest web-palette equivalents within a
  tolerance; higher shifts more colors.
- **WBMP** is 1-bit black/white with a dithering algorithm; Diffusion can seam
  across slice boundaries, and linking slices diffuses across linked slices.

### Color table (GIF and PNG-8)

The selected slice's color table appears in the **Color Table** panel; selecting
multiple slices with different tables shows "Mixed". Operations:

- **Sort**: Unsorted, Sort By Hue, Sort By Luminance, Sort By Popularity.
- **Add a new color** (New Color); the table becomes Custom when added with
  Ctrl/Command; new colors are locked.
- **Select colors** (click, Shift-click range, Ctrl/Command-click non-adjacent;
  or Eyedropper in the preview).
- **Shift a color** (double-click swatch to pick a new RGB value); **Shift/Unshift
  Selected Colors To/From Web Palette**; **Web Snap** tolerance.
- **Map colors to transparency** / unmap.
- **Lock/unlock colors** so they survive palette reduction.
- **Delete colors** (switches the table to Custom).
- **Save/Load Color Table**: `.act` (Adobe Color Table), `.aco` (Adobe Color
  Swatch), or a GIF's embedded table.

### Metadata

**Metadata** menu: **None** (smallest; JPEG still keeps the EXIF copyright
notice), **Copyright**, **Copyright and Contact Info**, **All Except Camera
Info**, **All**. Fully supported for JPEG; partially for GIF/PNG. Output conforms
to Metadata Working Group standards, so some JPEG metadata is written as EXIF/IIM
rather than XMP.

### Image Size while optimizing

The **Image Size** tab resizes during optimization: **Constrain Proportions**;
**Quality** (interpolation; Bicubic Sharper recommended when reducing); enter new
pixel dimensions or a percentage and **Apply**. (Help notes **Anti-Alias** and
**Clip To Artboard** are Illustrator-only; the Image Size features are unavailable
for SWF/SVG except Clip To Artboard — irrelevant to Photoshop.)

### Device Central (CS5 legacy)

The dialog's **Device Central** button previews the optimized file on mobile
devices. CS6 retains the entry point; Device Central is a CS5 artifact and
`Non-goal (Linux)` pending a substitute.

### Slices

Slices divide an image into smaller images reassembled in an HTML table or CSS
layers; each slice can carry its own URL and optimization settings. Saving uses
`Save For Web & Devices` to write each slice as a separate file and generate the
HTML/CSS.

- **Slice types by creation**: **user** (Slice tool), **layer-based**
  (`Layer > New Layer-based Slice`), **auto** (regenerated to fill the rest), and
  **subslice** (auto slices from overlaps).
- **Slice content types**: **Image**, **No Image** (empty table cell, can hold HTML
  text), and **Table** (per the slice-types topic).
- **Slice options** (`Slice Options` dialog / Slice Select options bar):
  **Type** (Image/No Image), **Name** (not for No Image), **Background Color**
  (None / Matte / White / Black / Other), **URL** and **Target** frame
  (`_blank`, `_self`, `_parent`, `_top`), **Message Text**, **Alt Tag**, dimensions
  **X/Y/W/H**, and (No Image) **HTML text** with **Text Is HTML** and cell
  alignment.
- **Modifying**: select, move/resize/snap (user slices only; not in the Save For
  Web dialog), **Divide** (horizontal/vertical, by slices or pixels), duplicate,
  copy/paste, **Combine**, stacking order, align/distribute, delete, **Lock
  Slice**, `View > Clear Slices`.
- **Linking**: in the Save For Web dialog, linked slices in GIF/PNG-8 share a
  palette and dither pattern to avoid seams. **Link Slices / Unlink Slice / Unlink
  All Slices** live on the Optimize menu.

### Output settings (HTML/CSS generation)

**Output Settings** dialog, reachable from **Other** in the Save Optimized Settings
menu or **Edit Output Settings** on the Optimize menu:

- **HTML**: Output XHTML, Tags Case, Attribute Case, Indent, Line Endings, Encoding
  (Photoshop always uses UTF-8), Include Comments, Always Add Alt Attribute,
  Always Quote Attributes, Close All Tags, Include Zero Margins On Body Tag.
- **Slices**: **Generate Table** (HTML table) vs. **Generate CSS** (stylesheet);
  Empty Cells (GIF, IMG W&H / GIF, TD W&H / NoWrap, TD W&H); TD W&H (Always /
  Never / Auto); Spacer Cells (Auto / Auto Bottom / Always / Always Bottom /
  Never); CSS **Referenced** (By ID / Inline / By Class); **Default Slice Naming**
  (document name, slice word, numbers/letters, rollover state, creation date,
  punctuation).
- **Background**: View Document As (Image / Background), Background Image, Color.
- **Saving Files**: File Naming (elements: document name, slice name, rollover
  state, trigger slice, creation date, slice number, punctuation, extension),
  Filename Compatibility (Windows / Mac OS / UNIX), Put Images In Folder, Copy
  Background Image When Saving.

The exported HTML title/copyright come from **File > File Info** (Document Title,
Copyright Notice).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `File > Save For Web & Devices` | Dialog | `Ctrl/Cmd+Shift+Alt+S` *(inferred)* | Optimization + optional HTML |
| Display tabs | Tab bar | — | Original / Optimized / 2-Up / 4-Up |
| Preview pop-up | Menu | — | Monitor / Legacy Macintosh / Windows / Document Profile; modem speed |
| Preset menu | Combo | — | Named optimization presets |
| Optimize pop-up menu | Menu | — | Repopulate Views, Optimize To File Size, Link/Unlink Slices, Edit Output Settings, Save/Delete Settings |
| Color Table panel | Panel | — | Sort/add/shift/map/lock/delete; save/load `.act`/`.aco` |
| Image Size tab | Tab | — | Resize during optimization |
| Layers tab | Tab | — | CSS layers (Illustrator-only; not Photoshop) |
| Metadata menu | Menu | — | None / Copyright / Copyright and Contact Info / All Except Camera Info / All |
| Save Optimized As | Dialog | — | Filename, Format (HTML+Images / Images Only / HTML Only), Settings, Slices |
| Output Settings | Dialog | — | HTML / Slices / Background / Saving Files sets |
| Slice tool / Slice Select tool | Toolbar | `C` (cycles with Crop) | Create/select/modify slices |
| `Layer > New Layer-based Slice` | Command | — | Slice from a layer |
| Slice Options | Dialog | — | Name, URL, target, alt, message, background, dimensions, HTML text |
| `View > Show > Slices` | Toggle | — | Show/hide slice boundaries |
| `View > Lock Slice` / `View > Clear Slices` | Command | — | Lock / delete all user+layer slices |
| `Edit/PS > Preferences > Guides, Grid & Slices` | Preference | `Ctrl/Cmd+K` | Show slice numbers, slice line color |
| Device Central button | Button | — | CS5-era mobile preview (Non-goal on Linux) |
| `File > File Info` | Dialog | `Ctrl/Cmd+Alt+Shift+I` | Document Title + Copyright for exported HTML |

## Parameters & ranges

| Control | Format | Type | Default | Range / options | Notes |
|---|---|---|---|---|---|
| Display tab | all | enum | Optimized | Original / Optimized / 2-Up / 4-Up | 4-Up choose a pane |
| Preset | all | enum | *(unnamed)* | GIF/JPEG/PNG presets | Editing a preset shows "Unnamed" |
| Format | all | enum | GIF *(inferred)* | GIF, JPEG, PNG-8, PNG-24, WBMP | SVG/SWF Illustrator-only |
| Quality | JPEG | int | 60 *(inferred)* | 0–100 (SFW scale) | Higher = better/larger |
| Optimized | JPEG | bool | Off | on / off | Smaller file; older browsers |
| Progressive | JPEG | bool | Off | on / off | Requires Optimized |
| Blur | JPEG | number | 0 | 0–2 (recommended 0.1–0.5) | Gaussian-Blur-like |
| Embed Color Profile | JPEG | bool | Off *(inferred)* | on / off | Preserves profile |
| Matte | JPEG/PNG-8/GIF | color | None | Eyedropper / Foreground / Background / White / Black / Other | Transparency simulation |
| Lossy | GIF | int | 0 | 0–~50 usable | Off with Interlace/Noise/Pattern |
| Color Reduction Method | GIF/PNG-8 | enum | Selective | Perceptual / Selective / Adaptive / Restrictive (Web) / Custom / B&W / Grayscale / Mac OS / Windows | — |
| Colors | GIF/PNG-8 | int | 256 | 2–256 | Lookup-table size |
| Dithering Method | GIF/PNG-8/WBMP | enum | Diffusion *(inferred)* | Diffusion / Pattern / Noise (GIF/PNG-8); + No Dither (WBMP) | — |
| Dither | GIF/PNG-8/WBMP | int percent | 100 *(inferred)* | 0–100 | Lower = smaller |
| Transparency | GIF/PNG-8/PNG-24 | bool | Off | on / off | PNG-24 supports 256 levels |
| Transparency Dither | GIF/PNG-8 | enum | No Transparency Dither | No / Diffusion / Pattern / Noise | When Transparency on |
| Interlace | GIF/PNG/WBMP | bool | Off | on / off | Larger file |
| Web Snap | GIF/PNG-8 | int percent | 0 | 0–100 | Higher shifts more |
| Metadata | all | enum | *(unresolved)* | None / Copyright / Copyright and Contact Info / All Except Camera Info / All | JPEG full; GIF/PNG partial |
| Convert to sRGB | all | bool | On | on / off | Only when non-sRGB profile |
| Image Size — dimensions | all | number | document size | pixels or percent | With Apply |
| Image Size — Constrain Proportions | all | bool | On | on / off | — |
| Image Size — Quality | all | enum | Bicubic *(inferred)* | Nearest Neighbor / Bilinear / Bicubic / Bicubic Sharper / Bicubic Smoother | Interpolation |
| Save Format | Save Optimized As | enum | HTML and Images | HTML and Images / Images Only / HTML Only | — |
| Slices | Save Optimized As | enum | All Slices | All Slices / Selected Slices | When slices exist |
| Slice Type | Slice Options | enum | Image | Image / No Image | Name unavailable for No Image |
| Background Color | Slice Options | enum/color | None | None / Matte / White / Black / Other | Not previewed in canvas |
| URL / Target | Slice Options | string/enum | — | URL; `_blank`/`_self`/`_parent`/`_top` | Image slices only |
| Message Text / Alt Tag | Slice Options | string | — | free text | Image slices; exported HTML |
| Slice dimensions X/Y/W/H | Slice Options | int px | slice rect | ≥0 | User slices |
| Text Is HTML | Slice Options | bool | Off | on / off | No Image HTML text |
| Cell Align H/V | Slice Options | enum | Default | Left/Center/Right; Top/Baseline/Middle/Bottom | No Image |
| Output XHTML | Output Settings | bool | Off | on / off | Forces tag/attribute case |
| Generate Table / Generate CSS | Output Settings | enum | Generate Table | Table / CSS | CSS Referenced: By ID/Inline/By Class |
| Empty Cells | Output Settings | enum | *(unresolved)* | GIF IMG W&H / GIF TD W&H / NoWrap TD W&H | Table layout |
| Spacer Cells | Output Settings | enum | Auto | Auto / Auto Bottom / Always / Always Bottom / Never | Prevents table break |
| Filename Compatibility | Output Settings | multi | *(unresolved)* | Windows / Mac OS / UNIX | — |
| Show Slice Numbers | Guides, Grid & Slices pref | bool | On *(inferred)* | on / off | — |
| Slice line color | Guides, Grid & Slices pref | color | *(unresolved)* | palette | Contrast color when selected |

## Algorithms & pipeline

### Optimization engine

```text
optimize(image, format, options, slice) -> encoded_bytes
  GIF/PNG-8: quantize (Color Reduction Method, Colors) -> palette/color table
             -> dither (method, %) -> optionally map transparency
             -> LZW (GIF) / deflate (PNG-8) with interlace flag
  PNG-24:    deflate RGBA (lossless), optional interlace
  JPEG:      optional blur -> DCT quantize by quality -> optional optimized
             Huffman / progressive scan
  WBMP:      1-bit threshold with dither
```

- **Quantization** (`GIF`/`PNG-8`): Perceptual weights by human sensitivity;
  Selective favors broad color areas + web colors (default); Adaptive samples the
  image spectrum; Restrictive (Web) uses the 216-color web-safe palette; Custom/
  fixed palettes are user- or preset-supplied. Implementation candidate:
  `imagequant` (`ARCH-011`).
- **Dithering**: Diffusion = error diffusion across adjacent pixels (Floyd–Steinberg
  family); Pattern = ordered halftone-like matrix; Noise = random without diffusion.
- **Transparency**: GIF is binary (1-bit) transparency; PNG-8 supports palette
  transparency; PNG-24 supports full alpha. With matte on, partially transparent
  pixels composite against the matte color; with Transparency off, fully and
  partially transparent pixels collapse to the matte.
- **4-Up "Repopulate Views"**: after a settings change, generate the three lower-
  quality variants from the current image by scaling the quality/settings
  parameters — a preview convenience, not an output format.
- **Optimize To File Size**: binary-search quality/palette size to meet a target
  byte count; **Start With** = Current Settings or Auto Select GIF/JPEG; **Use** =
  current slice / each slice / all slices.

### Slice model and export

```text
SliceSet = { user_slices, layer_slices, auto_slices (derived), subslices (derived) }
```

- Auto slices are regenerated whenever user/layer slices change; slice numbers
  follow left-to-right, top-to-bottom from the upper-left.
- On export, each Image slice is encoded with its own options (linked slices share
  palette/dither in GIF/PNG-8); No Image slices emit no image and may carry HTML
  text or a background color.
- Slice geometry (dividing, combining, stacking) mutates the slice table; the
  canvas pixels are unchanged.

### HTML/CSS generation

- **Generate Table**: emit an HTML `<table>` with one cell per slice, using the
  Empty Cells/Spacer Cells/TD W&H rules to survive imperfect alignment.
- **Generate CSS**: emit absolutely positioned `<div>`s (referenced By ID, Inline,
  or By Class) — a modern alternative to the table layout.
- **Saving Files**: filename templating from the naming elements; images placed in
  the chosen folder; background image copied if configured.
- **HTML options**: XHTML output, tag/attribute case, indentation, line endings,
  UTF-8 encoding, comments, alt attributes, attribute quoting, closing tags, zero
  body margins.
- **Title/copyright**: from File Info; copyright is added as an HTML comment and as
  image metadata, not visible in the browser.

### Image Size during optimization

Resize the source before encoding using the chosen interpolation; unlike a
document Image Size command, it does not touch the open document. Bicubic Sharper
is the recommended reduction filter.

## Rust module mapping

Proposals, on top of `ARCH-011` (`pictura_io::raster`) and `WF-003`:

- `pictura_io::web::WebFormat` — `Gif | Jpeg | Png8 | Png24 | Wbmp`; capability flags.
- `pictura_io::web::JpegOptions { quality, optimized, progressive, blur, matte,
  embed_profile }`.
- `pictura_io::web::IndexedOptions { lossy_gif, reduction: Reduction, colors: u16,
  dither: DitherMethod, dither_pct, transparency, matte, transparency_dither,
  interlace, web_snap }`.
- `pictura_io::web::Png24Options { transparency, interlace }`;
  `WbmpOptions { dither: DitherMethod, dither_pct }`.
- `pictura_io::web::ColorTable` — palette container; sort/add/shift/web-shift/map/
  lock/delete; `.act`/`.aco` load/save.
- `pictura_io::web::quantize` — wraps `imagequant` (candidate) for GIF/PNG-8.
- `pictura_io::web::optimize_to_size` — target-size search.
- `pictura_io::slices::SliceSet` — user/layer/auto/sub slices; `Slice { rect,
  content: Image | NoImage, name, url, target, alt, message, background, html_text,
  text_is_html, cell_align }`.
- `pictura_io::web::html::OutputSettings` — the HTML/Slices/Background/Saving Files
  option sets; `generate_html(slices, settings)` / `generate_css(...)`.
- `pictura_io::web::sfw::Session` — the dialog session: display mode, per-pane
  settings, 4-up repopulate, metadata choice, image-size resize, save plan.
- `pictura_shell::web::SaveForWebController` — binds the dialog to the optimizer and
  the Save Optimized As flow.

Types crossing the Qt boundary: `WebFormat` + option structs, `ColorTable` (as a
model), `SliceSet` (as a model), `OutputSettings`, and progress/preview bytes for
the panes.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `SaveForWebDialog` | `QDialog` | The whole SFW surface: tabs, panes, optimize/color-table/image-size panels, toolbar |
| `ImagePreviewPane` | `QWidget` (custom) | One of Original/Optimized/2-Up/4-Up; pan/zoom; annotation caption |
| `OptimizeOptionsPanel` | `QStackedWidget` | Per-format option pages (JPEG/GIF/PNG-8/PNG-24/WBMP) |
| `ColorTableWidget` | `QAbstractItemView` + model | Palette swatches, sort/add/shift/map/lock/delete, save/load |
| `SliceOverlay` | `QGraphicsView` layer | Draw/select/move/resize slices; badges; show/hide auto slices |
| `SliceOptionsDialog` | `QDialog` | Type, name, URL, target, alt, message, background, dimensions, HTML text |
| `OutputSettingsDialog` | `QDialog` | HTML / Slices / Background / Saving Files pages |
| `SaveOptimizedAsDialog` | `QFileDialog`-based | Format (HTML+Images/Images Only/HTML Only), Slices (All/Selected) |
| `MetadataCombo` | `QComboBox` | Metadata-saving level |
| `ImageSizePanel` | `QWidget` | Dimensions/percent, constrain, interpolation quality, Apply |
| `PreviewBrowserLauncher` | `QDesktopServices` | Open the optimized preview in a chosen browser |

Widgets over QML for the dense option panels; the multi-pane preview is a custom
`QWidget`/`QGraphicsView` because it needs precise per-pane hit-testing, slice
overlays, and live re-encoding on the UI thread with background workers. The
`ColorTableWidget` and `SliceSet` are model/view so the same models feed both the
dialog and the export writer.

## Data-model impact

- Save for Web is **non-mutating**: it reads the current composite (or per-slice
  regions) and writes files. It does not alter the document's layers, history, or
  saved path.
- **Slices are document data**: user and layer-based slices persist in the
  document and serialize into PSD image resource 1050/`Slices` (and into the
  slicing resources). Auto/sub slices are derived and not serialized. Slice fields
  (URL, target, alt, message, background, content type, HTML text) are part of the
  slice record.
- **Optimization presets** and output settings persist as **application/user
  presets** (files in the Photoshop Optimized Settings / Output Settings folders),
  not in the document.
- **Color tables** can be saved/loaded as external `.act`/`.aco` files.
- **Metadata** selected in SFW maps to the metadata engine (`ARCH-008`): JPEG gets
  full carriage; GIF/PNG partial.
- **Convert to sRGB** is a transient color conversion for the exported bytes only.
- Undo: slice creation/moves are undoable document edits (`03-tools/slice-tools.md`);
  the Save for Web dialog is not a document-history operation.

## Edge cases

- **16-bit → 8-bit**: SFW always converts to 8-bit (Help). Warn where CS6 does.
- **Non-sRGB profile**: Convert to sRGB defaults on; turning it off can shift
  browser colors.
- **GIF/PNG-8 >256 colors**: quantization is required; palette reduction trades
  quality for size.
- **Transparency + matte**: fully vs. partially transparent pixels behave
  differently; GIF is binary alpha; PNG-24 is full alpha.
- **Lossy GIF incompatibilities**: cannot combine with Interlace or Noise/Pattern
  dither.
- **Linked slices**: must share palette/dither in GIF/PNG-8; unlinking may create
  seams across boundaries.
- **Overlapping slices**: subslices are generated; stacking order decides which
  slice owns the overlap.
- **No Image slices**: not exported as images; HTML text layout can change the
  page; HTML tags are ignored unless Text Is HTML is set.
- **Slice numbering shifts**: editing the slice table renumbers; exported HTML must
  follow the current numbering.
- **Image Size during optimization**: does not change the document; repeated Apply
  compounds (operates on the current preview).
- **WBMP**: 1-bit only; browser support is effectively mobile-legacy.
- **Metadata**: some formats cannot carry all metadata; None strips everything but
  the JPEG EXIF copyright notice.
- **Device Central / SVG / SWF**: absent or Illustrator-only; do not present dead
  options in Photoshop.
- **Very large documents / many slices**: re-encoding on every settings change can
  be slow; debounce and use background workers with cancellation.
- **GPU unavailable**: optimization is CPU-side; previews still render.

## Parity acceptance criteria

- Given a document and `File > Save For Web & Devices`, the dialog shows
  Original/Optimized/2-Up/4-Up, and in 4-Up **Repopulate Views** regenerates the
  three non-selected panes from the current settings.
- Given a non-sRGB document with Convert to sRGB on, the exported JPEG/PNG carries
  sRGB-converted pixels and a matching profile (when Embed Color Profile is on).
- Given a GIF export at Colors = 32, Selective, Diffusion 100%, the output palette
  has ≤32 entries and the image meets a stated color-distance tolerance from the
  quantized reference.
- Given a GIF with Lossy > 0, the dialog disables/refuses Interlace and Noise/
  Pattern dither as documented.
- Given a PNG-24 export with Transparency on, up to 256 alpha levels are preserved
  and Matte is disabled.
- Given a JPEG export at Quality Q with Progressive on, the file is progressive and
  decodes to an image within the JPEG PSNR tolerance.
- Given a sliced document, `Save Optimized As` with Format = HTML and Images writes
  one image per Image slice plus an HTML file whose table/CSS layout reassembles the
  image; changing to Images Only omits the HTML.
- Given two linked GIF slices, both share a palette and dither pattern (no seam)
  within tolerance; unlinking allows independent palettes.
- Given Slice Options with URL, Target, Alt, and Message Text, the exported HTML
  contains a matching `<a href>`, `target`, `alt`, and status text.
- Given Output Settings Generate CSS / Referenced By ID, the exported layout uses
  absolutely positioned CSS rather than a table.
- Given the Image Size tab with a 50% resize and Apply, the exported file is half
  the source dimensions with the chosen interpolation, and the open document is
  unchanged.
- Given Metadata = None, the exported JPEG contains no metadata except the EXIF
  copyright notice; with All, all supported metadata is present.
- Given an optimized file, the annotation area reports a file size and download
  time consistent with the selected modem speed.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — the
  CS6 Help corpus, downloaded and text-extracted for this pass. Established: the
  **Save For Web & Devices** dialog layout and display options (Original/Optimized/
  2-Up/4-Up, Repopulate Views), pan/zoom, per-pane annotations with modem speed, and
  the four gamma-preview options; the full **Optimize an image for the web**
  workflow including Convert to sRGB, the Metadata levels, and the Save Optimized
  As Format/Slices options; **Save or delete optimization presets**; **Work with
  slices in the Save For Web & Devices dialog** (linking/unlinking, shared palette
  and dither, slice visibility/dimming); **Compress a web graphic to a specific
  file size** (Optimize To File Size, Start With, Use); **Resize artwork while
  optimizing** (Image Size tab, Constrain Proportions, Quality interpolation);
  **Generate CSS layers for web graphics** (Illustrator-only); **Preview optimized
  images in a web browser**; **Save a file to e-mail**; **Output settings for web
  graphics** (HTML, Slices, Background, Saving Files option sets in full); **Web
  graphics optimization options** for JPEG (Quality, Optimized, Progressive, Blur
  0.1–0.5, Embed Color Profile, Matte), GIF and PNG-8 (Lossy, Color Reduction
  methods and fixed palettes, Colors, Dithering methods and percentage,
  Transparency and Matte, Transparency Dithering, Interlace, Web Snap), **Optimize
  transparency in GIF and PNG images** (including the >50% rule and PNG-24
  multilevel transparency), **View/Customize the color table** (sort, add, select,
  shift, web-shift, map transparency, lock, delete, save/load `.act`/`.aco`), and
  **PNG-24** / **WBMP** options; the **WBMP** 1-bit note and the SWF/SVG
  Illustrator-only note; the family of **Slicing web pages** topics (slice types:
  user/layer-based/auto/subslice; content types Image/No Image/Table; create from
  guides or layers; promote/convert; view slices; **Modifying slice layout**:
  select, move/resize/snap, Divide, duplicate, copy/paste, combine, stacking,
  align/distribute, delete, lock, `View > Clear Slices`); the **HTML options for
  slices** topics (Slice Options for type, name, background color, URL/target,
  message/alt, No Image HTML text with Text Is HTML and cell alignment); the
  **Working with web graphics** chapter (rollover images, **Export to Zoomify**,
  hexadecimal color values); the `File > New > Device Central` and SFW Device
  Central (CS5) entry points; and the **Copy CSS from layers | Creative Cloud**
  note explicitly labelled Creative Cloud (not CS6).
- `https://html.duckduckgo.com/html/?q=Photoshop+CS6+New+Document+dialog+options+artboard+recent`
  — search-result page (snippet only); used only for the CS6 artboard context in
  `WF-002`, not for web export.

Internal cross-references (not external URLs): `docs/01-architecture/file-formats.md`
(quantization and codec crate candidates such as `imagequant`), `docs/03-tools/slice-tools.md`
(slice tool semantics), `docs/10-workflow-io/save-and-save-as.md` and
`docs/10-workflow-io/export-formats.md` (the shared option surface).

## Open questions

- **Copy CSS from layers is CC, not CS6.** The CS6 Help corpus includes a
  "Copy CSS from layers | Creative Cloud" topic. Confirm it is absent from the
  shipped CS6 build; if so, it must be excluded from CS6 parity and noted in
  `05-layers/layer-management-ui.md`.
- **Save for Web metadata default.** The default Metadata level (None vs. All) is
  not stated in the Help text. Resolve from a CS6 dialog pass.
- **Default optimization format and quality.** The default pane format (GIF vs.
  JPEG) and default JPEG quality in SFW are unstated (60 is a common value but
  unverified). Resolve from a CS6 install.
- **Lossy GIF exact range.** Help gives usable guidance (5–10, up to 50) but not
  the control's exact maximum. Confirm the slider's numeric range in CS6.
- **`.act`/`.aco` and GIF palette byte formats.** For round-trip parity, the exact
  Adobe Color Table and Color Swatch file structures need to be specified against
  the Adobe file-format docs (`ARCH-011`).
- **Slice PSD serialization keys.** The exact PSD image-resource keys for slices
  (for example 1050) and the slice-record layout should be confirmed against the
  PSD File Formats Specification and added to `ARCH-008`.
- **HTML output default set.** Which Output Settings preset CS6 ships as the default
  (Generate Table vs. CSS; Empty Cells/Spacer Cells defaults) is unverified.
- **Device Central substitute.** No Linux equivalent is chosen; the button's parity
  status is `Non-goal (Linux)` pending a decision.
- **WBMP relevance.** Whether WBMP is worth shipping at all on modern Linux is a
  product decision, not a sourced fact.
- **Qt6 details.** The multi-pane preview and color-table models need verification
  against Qt6 current docs before implementation.
