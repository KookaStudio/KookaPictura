# Proposal: route camera raw opens into Pictura Raw (#400)

## Why

The raw stages exist but nothing reaches them: `read_raw` (#396, #399) reads a
DNG or CR2 sensor plane, `demosaic` (#397) makes camera RGB, and
`as_shot_transform` / `to_pro_photo` (#398) map it to linear ProPhoto. File ▸
Open still sends a `.dng` down the raster route, which cannot read it. This is
the "Kooka cannot open raw files" gap in #86.

## What changes

- **Codec.** `pictura_codec::decode_raw_source` reads a raw container, checks
  that its colour can be mapped before demosaicing (a file without
  `ColorMatrix`, such as every CR2 today, is refused cheaply), demosaics with
  AHD, and keeps the as-shot white balance, default crop, orientation and
  `BaselineExposure`. `develop_raw` turns that camera RGB into 16-bit sRGB (the
  app's working RGB) at any temperature and tint: white balance and matrix
  (#398), `BaselineExposure`, default crop, orientation, then lcms2 from linear
  ProPhoto. `downscale_camera` makes the preview copy, and `raw_xmp` reads a
  DNG's embedded XMP packet; `crs_settings_from_xmp` reads its `crs:` Basic
  controls.
- **Pictura Raw.** `render_pictura_raw_rgb16` runs the develop controls on
  interleaved 16-bit RGB (the pipeline already works in `f32`).
- **Open route.** File ▸ Open (and every path that opens a document by path)
  detects a camera raw file by its signature, whatever its extension, before
  the raster route. The decode runs on a worker; then, interactively, the
  Pictura Raw dialog opens on the decoded image. OK develops at full size on a
  worker and opens a new 16-bit RGB document with one Background layer and one
  "Open" history state; Cancel opens nothing. A non-interactive open (control
  server, command line, self-test) skips the dialog and develops with the seed
  settings.
- **Seed settings.** Temperature and tint start at the as-shot white balance; a
  `<name>.xmp` sidecar's `crs:` Basic controls override them, else the DNG's
  embedded `crs:` controls.
- **Absolute white balance for raw.** In the raw dialog Temperature runs
  2000–50000 K on a reciprocal (mired) slider and Tint −150…+150; the colour
  stage applies them, and the relative Basic stage does not apply them again.
  JPEG/TIFF and layer sources keep the relative ±100 scale.
- **Unsupported files.** A raw file whose camera colour or container the
  codec does not support fails with "Could not open <name>: <reason>", opens no
  document, and does not crash.
- The Open dialog gains a "Camera Raw (*.DNG *.CR2)" filter.

## Out of scope (follow-ups)

- Shift+Open Image as a smart object whose double-click reopens the dialog (#1466).
- Workflow Options (space, depth, size; #633) and writing settings back (#632).
- Per-camera matrices for CR2 and other vendor files (#1465).

## Capabilities

- New: `codec/raw-develop`, `document/raw-open`.
- Modified (added requirement): `codec/pictura-raw`.

## Provenance

Orientation follows the TIFF 6.0 `Orientation` tag; `BaselineExposure`,
`DefaultCropOrigin/Size` and the XMP tag (700) follow the public DNG 1.6
specification. The reciprocal temperature slider follows the mired scale. No
code from the DNG SDK, dcraw, LibRaw or Adobe Camera Raw; the develop result's
match to ACR is approximate and not measured.
