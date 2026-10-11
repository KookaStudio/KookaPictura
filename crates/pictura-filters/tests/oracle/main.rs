//! ImageMagick differential oracle for `pictura_filters::apply` (tasks M6-E,
//! M7-C, M8-B, M9-B, M11-B).
//!
//! ImageMagick implements a handful of the same Blur / Sharpen / Noise / Other
//! / Stylize / Pixelate filters. This is a *sanity* oracle, not a parity
//! oracle: the exact integer math and convolution kernels are closed, and
//! the ImageMagick operators only approximate several reference paths. Each
//! filter with a faithful operator is diffed against the ImageMagick result
//! with `pictura_testkit::compare`; the tolerance and the reason for it are in
//! the table below and in `tests/README.md`.
//!
//! Filters with a faithful ImageMagick operator are run differentially
//! (`GaussianBlur`, `BoxBlur`, `Blur`, `BlurMore`, `Median`, `UnsharpMask`,
//! `Sharpen`, `SharpenMore`, `Maximum`, `Minimum`, `Offset` with
//! `wrap = true`, `Custom`, `Solarize`, `Mosaic`, `Displace` with Repeat Edge
//! Pixels, `ShapeBlur`). The rest are
//! covered by ImageMagick-independent property/known-value tests here and in
//! the module unit tests; the divergences that ruled out a differential test
//! are recorded in the table below and in `tests/README.md`. The M9 Distort
//! filters (`Twirl`, `Pinch`, `Spherize`, `Ripple`, `Wave`) were measured
//! against the closest ImageMagick operator (`-swirl` / `-implode` / `-wave`)
//! and classified no-equivalent; see the table. The M11 Distort filters
//! (`PolarCoordinates`, `Shear`, `ZigZag`, `OceanRipple`) were measured against
//! `-distort Polar`/`DePolar`, `-shear`, `-swirl` and `-wave` and are likewise
//! no-equivalent; see the table. The missing-filter-kernels batch
//! (`DustAndScratches`, `Extrude`, `Tiles`, `TraceContour`, `Wind`,
//! `SmartSharpen`) is no-equivalent as well and is covered by property and
//! known-value tests in the module unit tests; see the table.
//!
//! Regenerate/inspect a result manually with `scripts/filter_oracle.py`; see
//! `tests/README.md`.

mod common;
mod differentials;
mod displace;
mod distort_properties;
mod harness;
mod mapping;
mod pixelate_properties;
mod properties;
