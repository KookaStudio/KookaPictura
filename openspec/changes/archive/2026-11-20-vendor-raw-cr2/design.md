# Design: vendor-raw-cr2

## Module

Inside `crates/pictura-codec/src/dng/`, so the vendor readers share the #396
TIFF reader, lossless JPEG decoder and level normalization:

- `vendor.rs` — `RawFormat`, `sniff_raw`, `read_raw`, and the helpers later
  TIFF-based containers reuse: the Exif IFD and its maker note, and the black
  level measured from masked border photosites.
- `cr2.rs` — the CR2 reader.
- `ljpeg.rs` gains `frame(data)`, which reads the `SOF3` header without
  decoding, so the plane size is charged to the budget before allocation.

`DngImage` keeps its name: it is the normalized CFA plane every container
produces.

## CR2

- **Signature**: `II*\0`, then `CR` at byte 8 and major version 2 at byte 10.
  The little-endian 32-bit value at byte 12 is the raw IFD's offset.
- **Raw IFD**: compression 6, one strip (`StripOffsets`, `StripByteCounts`).
  The strip is one lossless JPEG frame of `W` samples per row and `C`
  components (1 to 4). The sensor plane's size is the raw IFD's
  `ImageWidth`/`ImageLength`, else the maker note's sensor size, else `W·C` ×
  `H`; only the sample counts must agree, because a frame row may carry two
  sensor rows (observed on the EOS 5DS: a 4448 × 2960 × 4 frame for an
  8896 × 5920 plane). A subsampled frame (small/medium raw) is refused by the
  decoder.
- **Slices**: tag `0xC640` holds `[n, w, last]`. The decoded sample stream
  fills `n` slices of width `w`, then one of width `last`, each top to bottom
  and row by row; `n·w + last` must equal the plane width. Without the tag the
  stream is the plane in row order.
- **Active area**: the maker note (Exif tag `0x927C`, an IFD whose offsets
  are relative to the file start) tag `0x00E0` holds signed 16-bit values;
  entries 5–8 are the left, top, right and bottom of the active area,
  inclusive. Without it the whole plane is active.
- **Black**: the mean of the masked photosites left of the active area (the
  rows above it when there is no left mask), per position in the 2 × 2 CFA
  phase measured from the active area's corner. Without a mask, 0.
- **White**: `2^precision − 1` of the JPEG frame.
- **CFA**: RGGB at the sensor plane's corner, reported from the active
  corner, so an odd border shifts the phase (observed: the EOS 450D's top
  border 23 gives GBRG, its two green positions' means agreeing within 0.1 %).
- **Tags**: `Make` (271), `Model` (272) and `Orientation` (274) from IFD0;
  `default_crop` is the whole active area; colour tags empty.

## Decisions

- **Decode is single-threaded**: a CR2 strip is one JPEG entropy stream.
- **Ponytails**: white is the precision ceiling, not the camera's saturation
  point (the maker note's ColorData white levels are per-version and not
  read), so highlights may normalize below 65535; the black is one mean per
  CFA position for the whole frame.
- **Real files** are opt-in through `PICTURA_RAW_CORPUS`, a directory of
  user-supplied camera files (not committable, `CONTRIBUTING.md` Assets).
