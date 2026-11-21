# Design: DNG output (#402)

## Owner decisions (2026-11-21)

- Both image kinds: mosaic (the default) and **Convert to Linear Image**.
- Entry point: ACR's **Save Image…** in the raw open's Pictura Raw dialog with
  a Save Options dialog; no File ▸ Save As DNG.
- Settings are embedded in DNGs we write only; `Export Settings To XMP` and
  in-place edits of an existing DNG are a follow-up.
- JPEG preview **and** Embed Original Raw File ship in this change.

## Codec: `dng/write/`

`write_dng(&DngWrite, progress, cancelled) -> Result<Option<Vec<u8>>>`
(`None` when `cancelled` turns true) builds the whole file in memory. A
DNG is a classic TIFF (≤ 4 GiB), so a file that would pass 4 GiB is refused
with `Invalid`; BigTIFF is not a DNG.

File layout, little-endian (`II*\0`):

```
header → IFD0 (thumbnail, NewSubFileType 1)  — colour tags, XMP, original raw
           SubIFDs → [ full image (NewSubFileType 0), JPEG preview (1)? ]
```

The reader's `raw_ifd` already skips IFD0 by its `NewSubFileType` and walks
the SubIFDs; `raw_xmp` reads IFD0's tag 700; `sniff_raw_prefix` sees
`DNGVersion` in IFD0's entry table. A small IFD builder (`tiff_out.rs`) lays
out each IFD after its out-of-line values and patches the SubIFD and
strip/tile offsets once every block's position is known.

- **Mosaic.** `DngImage` samples are written as they are (black 0, white
  65535, already cropped to the active area), so `read_dng(write_dng(x)) == x`
  exactly. `ponytail:` raw values and the original black/white levels are not
  kept; Embed Original Raw File is the lossless copy. Tiles are 256 × 256,
  lossless JPEG with predictor 1, each row encoded as two interleaved
  components of half width (the usual DNG trick so the predictor sees the
  same CFA colour), so the decoder's linear sample order is unchanged.
- **Linear.** `CameraRgb` (`PhotometricInterpretation` 34892,
  `SamplesPerPixel` 3), three-component lossless JPEG tiles.
- **Encoder.** A production encoder in `ljpeg_encode.rs` (a 64-bit bit
  writer, one Huffman table built per tile from its difference-category
  histogram, lengths limited to 16 bits, category 16 always present). Tiles
  encode in parallel with rayon; progress counts finished tiles.
- **Previews.** IFD0 holds an uncompressed 8-bit sRGB thumbnail (long side
  256). The optional preview IFD holds one baseline JPEG strip (compression 7,
  photometric YCbCr, `PreviewColorSpace` 2 = sRGB) encoded with the `image`
  crate's JPEG encoder at quality 90. The `image` crate is already in
  `Cargo.lock` (pulled by `jpeg2k`) with its JPEG feature; the codec depends on
  it directly with `default-features = false, features = ["jpeg"]`, adding no
  crate to the lock.
- **Original raw.** `OriginalRawFileData` (type UNDEFINED): big-endian fork
  length, `n + 1` offsets relative to the fork start, then `n` zlib streams of
  64 KiB input each (rayon, `flate2`), followed by an empty resource fork and
  zero file type / creator words.
- **Compatibility.** `ponytail:` approximate mapping of ACR's levels to DNG
  versions: 2.4 → 1.1, 4.1 → 1.2, 4.6 → 1.2, 5.4 → 1.3, 6.6 → 1.4 (the
  default). `DNGVersion` and `DNGBackwardVersion` both carry the level's
  version; `ForwardMatrix1/2` and `PreviewColorSpace` (DNG 1.2) are omitted
  below 1.2.

## Codec: linear reading

`read_linear_dng` shares the reader's container walk, layout and tile decode
(`decode_plane` gains a samples-per-pixel factor), normalizes each sample by
its own `BlackLevel` / `WhiteLevel` (one value or one per sample) after the
`LinearizationTable`, and crops to the active area. Black-level deltas and
repeat dimensions other than 1 × 1 are refused as `Unsupported` (`ponytail:`
our writer never emits them). `read_dng` keeps refusing a linear image (it is
the CFA reader). `decode_raw_source` picks the linear path when the
full-resolution IFD is `LinearRaw`, checks the colour first as before, and
skips the demosaic.

## App: Save Image

- **Bridge** (`open_jobs.rs`, no new bridge file): `raw_dng_default_path`,
  `raw_save_dng_start`, `raw_save_dng_running`, `raw_save_dng_progress`,
  `raw_save_dng_cancel`, `raw_save_dng_finish`. The job lives on
  `PendingRaw` (the dialog is modal over it, and the open's develop takes the
  pending source only after the dialog closes).
- **Worker** (`raw_open/save_dng.rs`). The pending source holds demosaiced
  camera RGB only, so the worker re-reads the file and checks its SHA-256
  against the pending key (a changed file is refused rather than written
  under the old settings). Mosaic: `read_raw`; linear: `decode_raw_source`
  (one more demosaic, no copy of the pending camera RGB). Thumbnail and
  Medium preview develop from a copy of the pending ≤ 1024 px preview
  source; Full Size develops the full camera RGB (demosaicing the mosaic when
  needed). The XMP is `pictura_raw_to_xmp` over the seed record with the
  white-balance label `#401` uses. The file is written to a temporary name
  in the target folder and renamed. A target that is the source file itself
  is refused.
- **Default name.** `<stem>.dng` in the source folder, or `<stem>-1.dng`,
  `<stem>-2.dng`, … when taken, so the default never names the source DNG.
- **Qt.** `RawSaveOptionsDialog` (`raw_save_options_dialog.{h,cpp}`) and
  `runRawSaveImage`, which asks before replacing an existing file, runs the
  job under `runCancellableJob`, and returns the message the Pictura Raw
  dialog shows in its `rawSaveStatus` label. Nested dialogs (Save Options,
  the replace question) run through a runner the coordinator sets to
  `runImportDialog`, so the test driver reaches them.
