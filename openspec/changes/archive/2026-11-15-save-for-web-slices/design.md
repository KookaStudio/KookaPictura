# Design

## Context

See `proposal.md` for motivation and `specs/` for the behavioural contract.

Current state that shapes the approach:

- `save_for_web_dialog.cpp` is 601 lines. Every setting lives in a widget and
  `encoded()` reads those widgets directly; `remembered()` persists one flat
  struct for the session. There is one Original and one Optimized pane.
- `pictura_codec::web` already owns palette reduction, GIF89a, and WBMP.
  PNG-8 and PNG-24 files, and JPEG, are written through Qt at the app boundary.
- `Document::slices` is `Vec<PsdRect>`; `pictura_render::resolve_slices` returns
  user and auto slices numbered in reading order, but user slices may overlap.
  Export therefore adds `resolve_export_slices`; the canvas overlay keeps the
  overlapping `resolve_slices` result. Slices carry no name, URL, or alt text;
  PSD resource 1050 is preserved verbatim, not written.
- `slice_count` / `slice_at` already expose resolved slices to C++ through
  `crop_group.rs`. `web_flattened` returns the sRGB composite as RGBA8888.
- `pictura_codec::raster_metadata` gathers EXIF/IPTC/XMP from the preserved
  image resources, and `embed_raster_metadata` splices them into JPEG and TIFF.
  PNG currently drops metadata.
- The SVG exporter shows the house pattern for a Qt-free text writer in
  `pictura_codec`; raster encoding stays at the Qt boundary.
- App sources are listed explicitly in `cmake/sources/documents.cmake`; a new
  `.cpp` is not compiled until it is added there.

## Goals / Non-Goals

**Goals:**

- Keep raster encoding and all file writing at the Qt app boundary; put the
  slice HTML/CSS text generation, `.act` parsing, palette mapping, and metadata
  filtering in Qt-free engine code with unit tests.
- Keep the existing no-slices Save behaviour byte-for-byte, so only sliced
  documents take the new path.
- Keep the dialog under its file-size target by extracting settings/encoding,
  the colour-table widget, and the Save Optimized As dialog into their own
  units.
- Make every new surface testable without modal dialogs: encoding, slice
  planning, HTML generation, and metadata embedding are callable functions.

**Non-Goals:**

- Slice names, URLs, targets, alt text, Slice Options, per-slice optimization,
  Selected Slices, link/unlink, Divide/Combine (tracked by #543).
- `.aco` tables, Web Snap, web-shift, and saved output presets.
- Zoom/pan and synchronized navigation in the preview panes.
- Editing the document or adding history states: Save for Web remains
  non-mutating.

## Decisions

### 1. Slice export activates only for user slices

A document always resolves to at least one auto slice, so keying the new path
off `resolve_slices().len() > 0` would change every Save into an HTML export.
The app instead checks for a resolved slice with `user_index >= 0` (or adds a
`has_user_slices` bridge). Unsliced documents keep today's `getSaveFileName` +
`saveTo` path, which also keeps the existing `tst_save_for_web` assertions
valid.

*Alternative:* always open Save Optimized As, closer to CS6 defaults, but it
changes existing behaviour and existing tests for no user benefit in this
issue.

### 2. HTML/CSS emitter in `pictura_codec::web::html`

HTML is a text document format with no Qt dependency, like SVG. The emitter is
a pure function over a list of placed cells and options, unit-tested with golden
strings. It HTML-escapes text and attribute values (`&`, `<`, `>`, `"`),
URL-encodes image paths, and sanitizes CSS ids to `[A-Za-z0-9_-]`, replacing
every other character with `_` and prefixing a digit-leading id with `s_`. The
C++ side supplies file names and scaled
rects and writes files. This avoids QString string-building rules in the GUI
layer and matches the existing `svg_from_view` precedent.

*Alternative:* build the HTML with `QString` in the save flow. Fewer bridge
types, but the layout logic would only be covered by Qt Test and would mix
string formatting into the dialog.

### 3. Export resolution and the table layout grid

`resolve_slices` preserves overlapping user slices for the canvas overlay, so it
does not tile. Before slice export, `pictura_render::resolve_export_slices`
clips user slices into non-overlapping export cells: later-created slices in
`Document::slices` order own any overlap (layer-style stacking), the remainder
of an earlier rectangle is split into rectangles, and auto slices fill the rest.
The result tiles the canvas, and `resolve_slices` is left unchanged.

The emitter collects the unique x and y edges, walks y-bands, and emits a `<td>`
only where an export cell starts, computing `colspan`/`rowspan` from the grid.
Every cell holds an exact-size `<img>`; no spacer GIFs are written. That
reproduces the PS shape (`id="Table_01"`, comments, charset/body attributes)
without the fragile spacer column. A zero-width or zero-height export cell is
omitted from both the images and the layout; the remaining cells still cover
the canvas.

The CSS variant keeps the same cells but writes a relative wrapper and
absolutely positioned divs, with a `<style>` block keyed by slice file stems
whose ids replace characters outside `[A-Za-z0-9_-]` with `_` and prefix a
digit-leading stem with `s_`.

*Alternative:* one `<td>` per grid intersection with no spans. Simpler, but it
writes more cells than the slice set and no longer maps one cell to one image.

### 4. Extract `WebSettings` and make encoding a free function

`save_for_web_settings.{h,cpp}` owns:

- `WebSettings` (format, palette options, JPEG options, PNG/WBMP options,
  locked/custom palette).
- Widget read/write helpers and the preset table.
- `encodeWebImage(const QImage&, const WebSettings&)`,
  `quantizeWebImage(...)`, palette helpers, and the remembered struct.

The dialog holds `WebSettings panes_[4]`, an `activePane_` index, and a
dialog-level `metadataLevel_`: metadata is one remembered choice, not a
per-pane setting. In Original/Optimized/2-Up the active pane is Optimized
(index 1); in 4-Up it is the clicked pane. A widget edit stores the active pane,
updates the session memory, and re-optimizes only that pane. `encoded()` becomes
a call to `encodeWebImage(sized(), panes_[activePane_])`; metadata is fetched
raw once at construction and embedded at save.

Done keeps the active pane's format and settings as the next default Optimized
pane and keeps the Metadata choice. The other 4-Up variants are not persisted.

*Alternative:* keep reading widgets and copy widget state between panes. More
signals blocked and harder to test; the struct gives slice export the same
encoding path for free.

### 5. 4-Up panes and Repopulate

The preview area becomes four labels in a 2x2 grid, each with an info label.
The dialog installs a mouse-press event filter on the three optimized labels to
select a pane, shown with a border. Pane 0 always shows the original. Save uses
the active pane.

Repopulate is deterministic and local: copy the active pane's settings to the
other panes, then for each optimize with a lower budget (JPEG quality -20/-40
clamped at 1; palette colours halved/quartered with a floor of 2). Adobe's
variant algorithm is unpublished, so the spec requires only "smaller or
equal and no larger", not specific values.

### 6. Colour table as palette post-processing plus two quantize modes

Display and per-entry edits (shift colour, sort, map/unmap transparency) are
post-processing on the `Indexed` result: change palette entries, reorder the
palette and remap indices, or set/clear the transparency entry. Deleting remaps
the affected indices to the nearest remaining entry. Unmapping clears the
transparency entry and leaves that colour opaque at its RGB. These operations
live in `pictura_codec::web` so they can be unit-tested and reused by the
widget.

Two engine paths cover requantization:

- `quantize_locked(rgba, w, h, options, locked)` uses an effective budget of
  `max(Colors, locked.len())` capped at 256, builds the reduction for
  `effective - locked.len()` entries, and appends the locked colours; pixels map
  to the nearest entry across the whole table. Adding a locked colour raises the
  effective table size up to 256; adding one when 256 colours are already
  locked is refused.
- `quantize_custom(rgba, w, h, options, table)` maps every pixel to the nearest
  supplied entry (the `.act` path). With Transparency on, clear pixels map to
  the table's transparent entry when it has one; with Transparency off, or when
  the table has no transparent entry, every pixel maps to its nearest entry
  after any partial alpha is laid over the matte.

The dialog keeps one palette for the active pane; there is no per-slice table
and no "Mixed" state in this change.

### 7. `.act` codec in `pictura_codec::web::act`

Read 768-byte tables and 772-byte tables. For 772, read the last four bytes as a
2-byte big-endian count and a 2-byte big-endian transparent index (`0xFFFF`
means none), truncating the table to the count; ignore a trailer whose count is
0 or greater than 256. Writing always uses the 772-byte form with the same
big-endian count and transparent-index trailer whenever the table has fewer than
256 entries or a transparency entry. Only a full 256-entry opaque table writes
the 768-byte form. A 16-colour table therefore round-trips its size and
transparency entry instead of padding to 256.

*Alternative:* parse in C++ with `QFile`. It is a file format, and the engine
unit test for a 772-byte trailer is cheap.

### 8. Metadata levels in the engine, embedding at the boundary

The dialog fetches the raw EXIF/IPTC/XMP bytes once at construction through
`web_metadata(view)`, with no level argument. `pictura_codec::web::sfw_metadata`
is the only metadata-filter home: `sfw_metadata(raw, level)` returns the
filtered `RasterMetadata`. At save, `web_embed_metadata(bytes, suffix, raw,
level)` runs that filter and embeds the result; `encodeWebImage` does not handle
metadata.

| Level | EXIF | IPTC | XMP |
|---|---|---|---|
| None | minimal copyright only (JPEG) | empty | empty |
| Copyright | empty | generated `2:116` | generated `dc:rights` |
| Copyright and Contact Info | empty | adds `2:80`, `2:110`, `2:118` | adds `dc:creator`, `photoshop:Credit` |
| All Except Camera Info | empty | original | original |
| All | original | original | original |

The generated copyright/contact packets are built from the parsed metadata with
`Iptc::set`/`encode_iptc` and a minimal XMP packet writer; the "all" levels use
the raw preserved resources so they round-trip. JPEG embedding reuses
`embed_raster_metadata`. PNG gets a new `png::attach_xmp` that inserts an `iTXt`
chunk (`XML:com.adobe.xmp`) before `IEND` using the existing CRC helper. GIF and
WBMP are left unchanged.

The minimal EXIF copyright for None is a TIFF header plus IFD0 with tag `0x8298`
only; it is emitted only when the document's parsed EXIF has that tag.

*Alternative:* drop the copyright exception and update the long-form doc. The
doc is the contract, and the extra writer is ~35 lines.

### 9. Save Optimized As is its own small dialog

`save_optimized_dialog.{h,cpp}` holds a file-name field with Browse, Format
(HTML and Images / Images Only / HTML Only), Slices (All Slices, fixed), and
Layout (Table / CSS), defaulting to `<base>.html`, HTML and Images, Table. It
returns the path and choices; the writer derives `<document>` from the chosen
stem.

Writes are atomic per file with `QSaveFile`. Slice images are computed once from
the scaled composite, with zero-area cells dropped, and written in reading
order. A failure returns false after the files already written stay in place
(matching the current single-file behaviour of not truncating).

### 10. Bridge additions

`web_export.rs` gains:

- `WebSliceCell { name: String, x: i32, y: i32, width: i32, height: i32 }` and
  `web_slice_html(cells: Vec<WebSliceCell>, canvas_width, canvas_height, title,
  image_dir, layout) -> String`.
- `web_act_parse(bytes) -> Vec<u8>` / `web_act_encode(packed) -> Vec<u8>` for
  the colour-table widget, packed as count + RGBA entries + transparent index.
- `web_metadata(view) -> WebMetadata` (raw EXIF/IPTC/XMP byte vectors), called
  once at dialog construction, and `web_embed_metadata(bytes, suffix, raw,
  level) -> Vec<u8>` at save.
- `web_quantize_locked(...)` / `web_quantize_custom(...)` variants of the
  existing palette bridge, or one entry point with an optional table.

Slice rects and numbers for export come from `resolve_export_slices`, exposed to
C++ through the `web_export` bridge; `slice_count`/`slice_at` keep backing the
canvas overlay.

### Architecture

```
SaveForWebDialog
  |-- WebSettings panes_[4], activePane_          (save_for_web_settings)
  |-- ColorTableWidget                            (save_for_web_color_table)
  |-- 4 preview panes (event-filter selection)
  |-- Metadata combo
  |
  +-- Save (no user slices) -> getSaveFileName -> saveTo
  |
  +-- Save (user slices) -> SaveOptimizedDialog
                              |
                              v
                        scale slice edges
                              |
                +-------------+-------------+
                |                           |
        crop + encodeWebImage        web_slice_html (Rust)
                |                           |
                v                           v
        images/<base>_NN.<ext>        <base>.html
```

## Risks / Trade-offs

- **HTML bytes are structural, not byte-identical to Photoshop.** The spec
  requires reassembly and the PS-shaped table attributes, not golden parity
  with a closed generator. Golden tests are our own expected strings.
- **4-Up variants are a deterministic approximation.** Adobe's Repopulate
  algorithm is unpublished; the spec only requires monotonic size/quality.
- **Metadata filtering is approximate for the copyright levels.** They are
  generated minimal packets, not field-by-field filters of the original
  packets; `All Except Camera Info` drops EXIF entirely rather than removing
  camera tags from it. The spec states this behavior so it is not claimed as
  full CS6 parity.
- **Colour-table palette vs. slices.** One table applies to the whole active
  pane, so a document whose slices would carry different CS6 palettes shows a
  single table. No "Mixed" state.
- **Tiny slices at small Image Sizes.** Scaling edges can collapse a slice to
  zero. The writer omits zero-area cells from both the images and the layout,
  and the remaining slices still cover the canvas; the table width/height
  attributes keep the canvas size.
- **QSaveFile per file.** A multi-file export can leave earlier files behind
  when a later write fails. The alternative (write to a temp directory then
  rename everything) adds a second atomicity layer for little user benefit.

## Migration Plan

No data migration. The new path is selected by the presence of user slices; no
document, session-store key, or PSD resource changes. The change is archived in
the same pull request that ships it, and the docs/ROADMAP commit carries
`TASK-ALLOWS-DOCS`.
