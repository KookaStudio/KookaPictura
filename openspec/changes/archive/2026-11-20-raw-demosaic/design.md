# Design: raw-demosaic

## Module

`crates/pictura-codec/src/dng/demosaic/`:

- `mod.rs` — `DemosaicMethod`, `CameraRgb`, `demosaic`, pattern checks and the
  budget charge.
- `bilinear.rs` — generic same-colour averaging.
- `ahd.rs` — the Bayer AHD pass over row bands.

## Bilinear

For each pixel and each channel it lacks, average the photosites of that
colour within the smallest square window (radius 1, 2, … up to the pattern
size) that holds one, clipped to the image, rounded to nearest. On an interior
Bayer pixel radius 1 is exactly classic bilinear (four orthogonal greens, two
or four reds/blues). An X-Trans-shaped pattern takes whichever radius first
holds the colour.

## AHD

Per band of 32 output rows, a padded `f32` window is built with a margin of 6
photosites on every side, mirrored at the image edge (`x' = −x`,
`x' = 2(w−1) − x`), which keeps the Bayer phase. Then, for each direction
(horizontal, vertical):

1. Green at red/blue sites with the paper's filter
   `[−¼, ½, ½, ½, −¼]` along the direction, clamped to the range of the two
   adjacent greens (`ponytail:` the clamp is a common guard against overshoot,
   not in the paper).
2. Red and blue everywhere by bilinear interpolation of the colour
   differences `R − G` and `B − G` against that direction's green.
3. CIELab from the candidate, treating camera RGB as linear sRGB/D65
   (`ponytail:` the camera matrix belongs to #398; the metric only ranks the
   two candidates).
4. Homogeneity: the count of the four orthogonal neighbours within the
   adaptive thresholds `εL`, `εC` of the paper (the smaller of the two
   directions' largest neighbour difference along their own axis).
5. Each pixel takes the direction whose homogeneity summed over its 3×3
   neighbourhood is larger, the mean of both on a tie.

The paper's final median filtering of colour differences is omitted
(`ponytail:` artifact reduction left for a later pass). Images under 8 × 8
use bilinear.

## Decisions

- **Output** is interleaved `Vec<u16>` RGB on the reader's 0–65535 scale; no
  white balance or matrix is applied (#398).
- **Patterns** missing one of red, green or blue are refused with
  `PsdError::Invalid`; the reader admits any 1–16 repeat with the three plane
  colours, but cannot guarantee each appears.
- **Budget**: `width · height · 6` bytes must fit `max_alloc_bytes`, and with
  the input plane `max_document_bytes`. Dense output is a `ponytail:` until the
  raw route adopts tiled planes.
- **Parallelism**: rows (bilinear) or bands (AHD) in parallel with rayon; the
  result is deterministic.
