# Design: dng-raw-reader

## Module

`crates/pictura-codec/src/dng/`:

- `tiff.rs` — header (classic `*` 42 and BigTIFF 43), IFD entries with typed
  values (integers, rationals, bytes, ASCII), byte-order aware, every offset
  and count checked against the file length.
- `ljpeg.rs` — lossless JPEG decoder: `DHT`, `SOF3`, `SOS`, `DRI`/`RSTn`,
  byte stuffing, difference category 16 meaning 32768 with no extra bits (the
  DNG convention), point transform. The decoded samples fill the destination
  tile in order, row by row, so an `N`-component frame of width `W/N` maps
  onto a tile of width `W`.
- `levels.rs` — linearization, black and white normalization, active-area crop.
- `mod.rs` — IFD selection, strip/tile layout, the public `DngImage`.

The existing `raster_metadata/tiff.rs` is a metadata merger with private
helpers built for rewriting IFD0; it is not reused.

## Output

`DngImage::samples` is a `Vec<u16>` of the active area, row-major, one sample
per photosite, with `round((L(v) − black(x, y)) · 65535 / (white − black(x, y)))`
clamped to 0–65535. `L` is the linearization table (the identity when absent,
out-of-range indices clamp to the last entry). `black(x, y)` is
`BlackLevel[y mod rows][x mod cols] + DeltaV[y] + DeltaH[x]` with `x`, `y`
relative to the active area. 16 bits keeps the plane at two bytes per
photosite for the demosaic; the normalized scale makes the levels the
reader's job, not every later stage's.

## Decode

The full image is decoded into a raw `u16` buffer (tile rows decoded in
parallel with rayon, each band its own disjoint slice), then normalized into
the cropped output in parallel by row. Both buffers are charged to the budget
before either is allocated. A lossless JPEG frame larger than its tile is
refused, so a tile can never be made to decode more than its own area.

## Decisions

- **Errors** reuse `PsdError` (`Unsupported`, `Invalid`, `Truncated`), the
  crate's error type, rather than a DNG-only enum.
- **4-colour CFAs** (CMYG) and `CFALayout` other than 1 are refused: the
  later stages are RGB.
- **The real-file check** is opt-in: `PICTURA_DNG_CORPUS` names a directory
  holding the user-supplied Leica M9 samples `L1.dng` and `L2.dng`, which are
  camera-authored and not committable (`CONTRIBUTING.md`, Assets). Its expected
  values come from a separate Python parse of the files, not from this reader.
