# Test plan

This plan covers `raw-demosaic` (#397): the new capability `codec/raw-demosaic`.

**Fixtures.** Planes are built in the test as `DngImage` values: a random
plane (fixed LCG seed), or an RGB image mosaiced through the pattern. The
patterns are the four Bayer phases and a 6×6 X-Trans-shaped layout. The
real-file row uses the user's Leica M9 samples, opt-in through
`PICTURA_DNG_CORPUS`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `demosaic_identity` | Random 40×30 plane under RGGB, BGGR, GRBG, GBRG and X-Trans; demosaic with Bilinear and AHD | Each pixel's own CFA channel equals its sample | `pictura-codec::sampled_channels_are_kept` |
| `demosaic_flat` | Mosaic (1000, 20000, 5000) under each pattern; demosaic with both methods | Every pixel is (1000, 20000, 5000) | `pictura-codec::flat_field_is_unchanged` |
| `demosaic_gradient` | Mosaic a grey ramp along x and along y, 48×40, each Bayer phase; both methods | Channels non-decreasing along the ramp; within 1 of the ramp 3 px from the border | `pictura-codec::gradient_stays_monotonic` |
| `demosaic_edge` | 40×40 grey with a vertical then a horizontal step edge 4000→60000, each Bayer phase; both methods | AHD's max deviation from grey within 2 px of the edge < ¼ of bilinear's | `pictura-codec::ahd_reduces_false_colour_at_edges` |
| `demosaic_fallback` | AHD on X-Trans 24×24, and Bayer 1×1, 3×2, 7×7 | Equals Bilinear, no panic | `pictura-codec::ahd_falls_back_to_bilinear` |
| `demosaic_refused` | Pattern [R, G, G, R]; valid plane with `max_alloc_bytes` below `w·h·6` | `PsdError::Invalid` each | `pictura-codec::invalid_patterns_and_budgets_are_refused` |
| `demosaic_m9_corpus` | `PICTURA_DNG_CORPUS=~/Downloads cargo test -p pictura-codec --test dng_demosaic leica` | 5216×3472, sampled channels kept, both methods | `pictura-codec::leica_samples_demosaic_when_present` |
