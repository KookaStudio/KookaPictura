# Design: raw open route (#400)

## Owner decisions (2026-11-20)

- Dialog first, new document: decode → Pictura Raw dialog → OK opens a new
  document, Cancel opens nothing. Non-interactive opens skip the dialog.
  Shift+Open Image as a smart object is a follow-up.
- Temperature and Tint are absolute for a raw source (kelvin and tint, seeded
  from the as-shot white balance); other sources keep the relative scale.
- The document is 16-bit RGB in the working RGB. The app's working RGB is
  fixed at sRGB (Color Settings shows it as a label), so it is 16-bit sRGB.
- A file without a usable `ColorMatrix` (every CR2 today) is refused with a
  clear message; only DNG opens until per-camera matrices land.

## Flow

```
C++ startOpenDocumentAtPath(path)
  raw_sniff_path(path)            reads a 256 KiB prefix, sniff_raw
  └─ startOpenRawPath             loading tab, OpenKind::Raster
       open_raw_start(view)       worker: read file → decode_raw_source
         "raw"   → view keeps PendingRaw (source + preview copy + seed text)
         "error:<reason>"         → existing failure path ("Could not open …")
  onOpenFinished("raw")
    interactive  → PicturaRawDialog(raw mode) via runImportDialog
                     OK     → open_raw_develop_start(view, settings text)
                     Cancel → cancelOpen(index): tab removed, Cancelled
    otherwise    → open_raw_develop_start(view, raw_seed_settings(view))
  open_raw_develop_start          worker: develop_raw → render_pictura_raw_rgb16
                                  → 16-bit document → "ok" (installs as usual)
```

The two-stage job reuses `open_jobs::start` / `finish_on_gui`: the decode
stage's worker returns no document and the status `"raw"`, after it has queued
the `PendingRaw` onto the view (the same queued closure, checked against the
request id, so a cancelled or stale request drops it). The develop stage is a
normal `start` job that installs an `OpenedDocument`.

## Codec (`pictura-codec/src/dng/develop.rs`)

- `RawSource { camera: CameraRgb, color: DngColor, crop: DngCrop, orientation,
  make, model, as_shot: WhiteBalance }`.
- `decode_raw_source(bytes, budget)`: `read_raw`, then `as_shot_transform` on
  the colour tags **before** `demosaic` so an unsupported file fails cheaply,
  then AHD. The refusal message names the camera when the file does
  ("camera colour for Canon EOS 450D is not supported yet").
- `develop_raw(camera: CameraRgb, &RawLook, wb) -> Rgb16`: takes the camera RGB
  by value (the full develop moves it out of the pending source; the preview
  passes a clone of its small copy), `white_balance_transform` → `to_pro_photo` → multiply by
  `2^BaselineExposure` (clip) → default crop (rounded to whole pixels, clamped
  to the image) → orientation (TIFF 1–8) → `pictura_color::convert` linear
  ProPhoto → sRGB, 16-bit, relative colorimetric with BPC.
- `downscale_camera(&CameraRgb, max_dim)`: box average so the long side is at
  most `max_dim`; the dialog's preview develops from this copy (the crop is
  scaled with it).
- `raw_xmp(bytes)`: IFD0 tag 700 bytes. `crs_settings_from_xmp(packet)` is the
  public face of `crs_xmp::parse_crs`.

## Pictura Raw 16-bit entry (`pictura-adjust`)

`render_pictura_raw_rgb16(width, height, &[u16], &settings) -> Vec<u16>` loads
the `f32` planes from 16-bit samples and stores them back at 16 bits; the
8-bit `render_pictura_raw` shares the stage code. The default settings are a
sample-identical no-op, as for 8-bit.

## App (Rust, `cxxqt_object/raw_open.rs`)

- `PendingRaw { path, source: RawSource, preview: CameraRgb, seed: String }`
  on `PictureViewRust` (`Option<Box<…>>`), dropped on develop, cancel, or the
  view's next open.
- `raw_sniff_path(path) -> bool`.
- `open_raw_start(view, path)`, `open_raw_develop_start(view, text)`.
- `raw_seed_settings(view) -> QString`, `raw_preview(view, text, max_dim) ->
  QImage`, `raw_as_shot(view) -> [kelvin, tint]` for the dialog's reset.
- The develop splits the settings: `Temp`/`Tint` go to the colour stage (an
  unset value means as-shot), and are cleared before the relative stage runs.
- Seed: as-shot kelvin and tint, rounded to whole numbers; then the sidecar
  `<stem>.xmp` beside the file, else the embedded packet, supplies any `crs:`
  Basic control it sets (its `Temperature` and `Tint` taken as absolute).
- The document: `NativeRender`-style — 16-bit planar samples retained as the
  native source and the layer's source channels, an 8-bit display derived by
  `>> 8` rounding, opaque alpha, named after the file stem, one "Open" state,
  path set to the raw file and `source_format` `"raw"`, so Save defaults to a
  new PSD rather than overwriting the raw file.

## App (C++)

- `ImportSaveCoordinator::startOpenRawPath`, a `rawOpens_` map view →
  interactive flag, and `onOpenFinished` handling `"raw"`. The dialog runs
  from a zero-delay timer (never inside the signal emission).
- `PicturaRawDialog` gains a raw-source constructor flag: the preview pane
  calls `raw_preview`, the Basic panel shows Temperature 2000–50000 K on a
  reciprocal slider and Tint −150…+150, and its Default/Auto reset returns them
  to the as-shot values.
- `AcrSlider::setReciprocalScale(bool)`: the handle position is linear in
  1/value (mired), so the daylight range is not squeezed into 7 % of the track.
- Open filters: "Camera Raw (*.DNG *.CR2)" and both in All Formats. The
  `raw` suffix stays Photoshop Raw; only the signature routes a camera raw.

## Ponytails

- Signature sniffed from a 256 KiB prefix; a DNG whose IFD0 lies beyond it
  takes the raster route (and fails as before).
- Preview box-averages camera RGB before the white balance (linear, so it is
  exact up to rounding) and develops on the GUI thread at ≤ 1024 px.
- The 8-bit display plane is `>> 8` of the 16-bit samples (no dither).
- No Workflow Options; output is always 16-bit sRGB.

## Risks

- Full-size develop memory: a 50 MP camera RGB is 300 MB, converted in place
  to ProPhoto, then the sRGB output 300 MB and the document planes. The
  pending source is moved into the develop, so it is not held twice.
