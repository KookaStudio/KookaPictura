# Design: export-as-dialog

## Context

`ExportAsDialog` today is a `QFormLayout` with a format combo, a quality slider,
and a scale spin box; `exportAsFromView` runs it, then a `QFileDialog`, then
`export_image`. The CC dialog inverts that: one modal dialog holds the settings
*and* the preview and export targets, and `Export…` opens the file/folder picker.

The PNG-8 and GIF encoders already exist (`pictura_codec::web::quantize`,
`encode_gif`) and are reachable from C++ through `web_export.rs`
(`web_quantize`, `web_encode_gif`, `web_flattened`). Save for Web already drives
them; Export As reuses the same bridge.

## D1. The dialog takes images, not a view

`ExportAsDialog` is constructed with the already-flattened targets:

```cpp
struct ExportEntry {
    QString name;    // document base name, or a layer name
    QString kind;    // "document" | "group" | "pixel" | "type" | "shape" | ...
    QImage image;    // straight-alpha RGBA8888, full target size
};
ExportAsDialog(const QVector<ExportEntry>& entries, QWidget* parent);
```

This mirrors `SaveForWebDialog(const QImage&, QWidget*)`: the dialog is pure Qt
and directly testable (`tst_export_as`) without a `PictureView`, and the bridge
work stays in `exportAsFromView`. The dialog exposes `encoded(int entry)`,
`sized(int entry)`, `saveTo(int entry, const QString& path)`, `suffix()`, and
`checkedEntries()` so tests never open a file dialog.

## D2. Building the targets

`exportAsFromView(parent, view)`:

1. Flatten the document with the existing `web_flattened(view)` → entry 0
   (`name` = document base name, `kind` = `"document"`).
2. For each **top-level** layer index `i` (`view->layer_count()`), skip
   `layer_kind(i) == "adjustment"`; add an entry with `layer_name(i)` and the
   image from a new bridge `layer_flattened(view, i)`.

`layer_flattened(view, i)` (Rust, `web_export.rs`):

- Clone the document, replace `layers` with `[layers[i].clone()]` (a group keeps
  its `children`), composite with `composite_active`, convert to sRGB RGBA.
- Crop to the layer's bounds: `layer.rect` for a leaf, the union of the
  subtree's `rect`s for a group; clamp to the document; fall back to the full
  canvas when the union is empty.
- `ponytail:` layer effects and clipping extend past `rect`; the crop uses the
  layer rect, not the effect-expanded bounds. A layer's own mask/blend is
  applied by the scratch composite.

Adjustment layers are excluded because a scratch composite of one has no pixels
(it modulates layers below). This matches CC's list.

## D3. File Settings

| Format | Value | Controls |
|---|---|---|
| PNG | `PNG` | Transparency checkbox |
| JPG | `JPG` | Quality slider (1–100, default 90) |
| PNG-8 | `PNG8` | Transparency checkbox |
| GIF | `GIF` | Transparency checkbox |

PNG / JPG write through the Qt edge (`QImageWriter`), matching Save for Web;
PNG-8 quantizes with `web_quantize` (Selective, 256 colours, no dither) and
writes the indexed `QImage` as PNG; GIF quantizes then `web_encode_gif`. The
suffixes are `png` / `jpg` / `png` / `gif`.

## D4. Image Size and Resample

Width and Height are linked by a chain toggle (default on). Scale (percent)
drives both; editing Width or Height recomputes the other and the scale. The
output image is `width × height`.

Resample maps the CC menu onto Qt's two filters — Qt has no bicubic family:

| CC option | Qt |
|---|---|
| Bicubic | Smooth |
| Bicubic Automatic | Smooth |
| Bicubic Sharper | Smooth |
| Bicubic Smoother | Smooth |
| Bilinear | Smooth |
| Nearest Neighbor | Fast |
| Preserve Details | Smooth |

`ponytail:` only Nearest Neighbor vs. everything-else is distinguishable; the
bicubic family is approximated by Qt's smooth filter.

## D5. Canvas Size

The output canvas is `Canvas Width × Canvas Height`. The resampled image is
placed at the **top-left** of the canvas; a larger canvas pads with transparency
on the right/bottom, a smaller one crops. **Reset** sets the canvas back to the
current image size. Default canvas = document size. The final `QImage` is
`canvas`-sized before encoding.

`ponytail:` the CC anchor is top-left here (inferred); a centre/anchor grid is
out of scope.

## D6. Preview, zoom, and the size estimate

The preview is a `QLabel` inside a `QScrollArea`, redrawn from the selected
row's image after sizing and canvas. Zoom controls: **Fit**, **1:1**, `+`, `-`,
and a percent readout; the label scales by the zoom factor.

The output-size estimate (CC's bottom-right `716.4 KB`) requires a
**full-resolution encode on every change**. Measured on this machine (release):
12 MP Selective+Diffusion quantize ≈ 0.9 s, GIF encode ≈ 0.27 s, 24 MP ≈ 1.8 s
+ 0.53 s. That is too costly to run synchronously on each spinbox tick, so:

- The estimate is computed on a **300 ms debounce** (a `QTimer`).
- Above **4 MP** (document pixels) it is **omitted**; the label shows only the
  output dimensions.
- `ponytail:` no background thread; a large document simply has no size readout.

## D7. Export targets and naming

- Exactly one row checked: `QFileDialog::getSaveFileName`, default
  `<document folder>/<name>.<suffix>`.
- More than one row checked: `QFileDialog::getExistingDirectory`, then write
  each row to `<dir>/<sanitized name>.<suffix>`; the document uses the document
  base name, a layer uses its layer name (filesystem-sanitized, de-duplicated
  with a numeric suffix on collision).
- The document entry is checked by default; checking a layer adds it.

## D8. Contract and tests

- `docs/10-workflow-io/export-formats.md` gains an **Export As (CC)** section
  (the surface, the format set, Image Size / Canvas Size, per-layer output) and
  its provenance; `TASK-ALLOWS-DOCS`.
- The self-test check `lpr_raster_export` (code 532) drops its two
  `ExportAsDialog` quality-gating assertions (the dialog shape changed); the
  remaining Save / Quick Export assertions stay. Exit code 532 is kept — the
  check shrinks, it is not retired.
- New Qt Test `tst_export_as`: formats and suffixes, PNG-8/GIF round-trip,
  Transparency/Quality gating, Image Size linking + Resample, Canvas Size +
  Reset, the layer list checkboxes, and `saveTo` byte equality. It links
  `pictura_shell` + `Qt6::Test` like `tst_save_for_web`.
