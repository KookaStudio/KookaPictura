## Why

`read_psd` accepts only bit depth 8 (and depth 1 for Bitmap), so a 16-bit PSD —
the common depth for photography — is refused with `PsdError::Unsupported` and
will not open at all (`crates/pictura-codec/src/read.rs:49`). `PixelBuffer` is
`Vec<u8>`, so the composite has no 16- or 32-bit sample representation. This is
the depth half of roadmap P4 (gap G4): open the files now by normalizing them to
the 8-bit working model, and be explicit that the save is lossy in depth.

## What Changes

- `read_psd` accepts bit depth 16 (big-endian `u16` samples, row byte stride
  `2 * width`) and bit depth 32 (big-endian `f32`, stride `4 * width`) for the
  Grayscale/RGB/CMYK/Lab modes, beside the existing 8 (all modes) and 1
  (Bitmap). It narrows every sample to 8-bit — depth 16 as `sample >> 8`, depth
  32 as `clamp(trunc(sample * 256), 0, 255)` — sets the working `depth` to
  `Eight`, records `Document.source_depth = Some(BitDepth::Sixteen |
  ThirtyTwo)`, and only then applies the existing color-mode normalization, so a
  16-bit CMYK file is narrowed and then converted CMYK→RGB.
- The row byte stride becomes depth-aware everywhere it is computed
  (`row_bytes(width, depth) = ceil(width * depth / 8)`): the raw composite
  branch, the per-scanline RLE byte counts (whose values are still counts of
  *compressed* bytes), and `planar_len`. ZIP-with-prediction becomes depth-aware:
  depth 8 keeps the byte-wise delta, depth 16 runs the delta per big-endian
  `u16` (mod 2^16), and depth 32 reverses the 4-byte-plane shuffle and runs a
  byte-wise delta, matching `psd-tools` and `ag-psd`.
- Depth-16/32 **layer** channels (color, transparency `-1`, mask `-2`) are
  decoded at the document depth and narrowed to 8-bit planes, mirroring the
  shipped depth-1 Bitmap expansion, so layer and composite paths agree.
- `write_psd` is unchanged: it keeps writing only 8-bit RGB/Grayscale. An
  open→save of a 16/32-bit file therefore saves as 8-bit — a documented,
  lossy-in-depth save, no longer silent.
- The application reports it: a `depth_notice()` invokable mirroring
  `mode_notice`, shown in the status bar after a successful open.
- New fixtures (a flat 16-bit and a flat 32-bit RGB PSD) and a `psd-tools`
  oracle proving the two narrowing formulas exactly, plus hand-built Rust files
  for the RLE row stride, the ZIP/ZIP-with-prediction layout, and layer
  narrowing.
- **BREAKING**: none at the consumer API; `Document` gains one `Option` field
  whose default is `None`.
- Deferred and still `PsdError::Unsupported`: Bitmap and Indexed at 16/32; the
  depth-32 HDR tone mapping (only the display-referred raw narrowing is read);
  and re-encoding a document back to its source depth on save.

## Capabilities

### New Capabilities

- `psd-bit-depth`: reading bit depths 16 and 32, the big-endian sample layouts
  and their 8-bit narrowing conversions, the depth-aware row stride and
  prediction, the `source_depth` record and the lossy-in-depth save, layer
  narrowing, the application notice, and the depth fixtures/oracle.

### Modified Capabilities

- `psd-codec`: the header-validation requirement widens the accepted depth set
  to 16/32 for Grayscale/RGB/CMYK/Lab, and the composite-read requirement makes
  the raw/RLE row stride and the ZIP-with-prediction step depth-aware.
- `psd-color-modes`: the non-RGB normalization requirement no longer rejects
  depth 16/32; those depths are read and normalized per `psd-bit-depth` instead.

## Impact

- `crates/pictura-core/src/lib.rs`: new `Document.source_depth` field
  (`Option<BitDepth>`).
- `crates/pictura-codec/src/read.rs`: depth gate, `row_bytes`, depth-aware
  prediction, sample narrowing for the composite and every layer channel.
- `crates/pictura-codec/src/color_mode.rs` (or a small depth module): the
  `u16`→8 and `f32`→8 narrowing helpers, each with a `ponytail:` ceiling.
- `crates/pictura-codec/src/tests/depth.rs` (new), `tests/psd/depth_oracle.rs`
  (new), `tests/fixtures/{rgb16,rgb32}.psd`, `scripts/generate-fixtures.py`.
- `crates/pictura-app`: a `depth_notice` invokable, the status-bar notice in
  `frame.cpp`, and one self-test check (exit code 298).
- No new external dependency; the narrowing is std-only.
