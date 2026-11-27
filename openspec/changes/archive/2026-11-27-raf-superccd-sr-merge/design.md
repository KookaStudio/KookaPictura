# Design: SuperCCD SR merge (#1648)

## Where the R frame is

Measured on the three SR sample files (raw.pixls.us, kept outside the repo):

| Body | `0x64` (S data) | `0x80` (R data) | Row stride |
|---|---|---|---|
| S3 Pro, S5 Pro | first section | second section, right after the first | width |
| F700 | first section | `0x64` + width × 2 bytes | 2 × width (`0x0130` top bit) |

So the R frame is always at header `0x80` with the S frame's row stride
(LibRaw's `shot_select = 1` reads `100 + 28`). The data is bounded by the
file: R rows a damaged file does not hold are marked missing and keep their
S samples.

The second directory (`0x78`) repeats the records with the R frame's own
levels; its black (`0x4000`) is used for R. LibRaw only trusts `0x78` when
the thumbnail offset (`0x54`) is above 120; the S2 Pro's header is shorter
and its `0x78` holds JPEG bytes, so the guard is ported too.

Other bodies with a second frame (S200EXR, HS20EXR, HS50EXR, X10, XF1,
X-S1) have upright layouts and stay on their primary frame: the merge is
limited to SuperCCD-lattice word sections, which are the SR bodies.

## Merge (`dng/raf_sr.rs`)

Both frames are repacked onto LibRaw's grid with the same active area, so
they share the grid CFA. Per sample, with `s` and `r` the black-subtracted
values over the S range (`white − black`):

- **Ratio.** For each CFA colour, `k` is the median of `s / r` over samples
  with `s` in [0.25, 0.75] and `r` at least 16 counts; fewer than 256 such
  samples fall back to the nominal ratio, `0x9200`'s first value in the S
  directory over that in the R one (16 on all three bodies), else 16.
  Measured medians: S3 Pro 16.2, S5 Pro 17.0, F700 15.1. Large grids are
  sampled at a fixed stride so the fit stays deterministic and bounded.
- **Blend.** `m = s` below 0.80, `m = k·r` at and above 0.95, linear in
  between; a missing R sample keeps `s`. A clipped S sample is a lower
  bound, so there `m` is at least `s`: an R photosite beside a clipped S one
  can read darker, and must not pull the highlight below white.
- **Headroom.** `h = max(1, max m)`; samples are `round(65535 · m / h)` and
  `BaselineExposure` grows by `log2 h`. A scene that never clips keeps
  `h = 1` and the S frame unchanged.

`ponytail:` the R photosites sit beside, not on, the S ones; the merge
treats them as co-sited, so high-contrast edges at the clip can fringe.

## Exposure before the clip

`develop_pixels` already splits the absolute white balance out of the
Pictura Raw settings. It now also takes `basic.exposure` out and adds it to
the look's `BaselineExposure` before `develop_raw`, which scales linear
ProPhoto and clips. A gain commutes with the linear colour conversion and
the clamp at 0, so pixels below the clip develop as the relative Exposure
did (within rounding); pixels above it are recovered instead of lost.
