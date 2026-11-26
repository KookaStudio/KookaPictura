## 1. Regression coverage

- [x] 1.1 Add Rust coverage for cubic modes, tiled parity using `tiled_image_size_matches_the_dense_resampler`, extended-range float planes, disabled effects, and rounded proportional style resizing.
- [x] 1.2 Add Qt coverage for `imageSizeResamplingChoices` through dialog selection and command replay, style choice and rounded targets, Auto Resolution, and physical column and pica units.

## 2. Implement Image Size behavior

- [x] 2.1 Add cubic modes and shared direction selection, and preserve extended-range f32 samples.
- [x] 2.2 Wire Scale Styles to the resize transaction and recorded command, scaling stored supported effects including disabled ones without changing Layer > Scale Effects behavior; apply the documented common-interval proportionality rule.
- [x] 2.3 Enable Auto Resolution and physical pica/column quantities with editable persistent column preferences; keep Type and New Document Preset Resolutions disabled.

## 3. Prepare for shipment

- [ ] 3.1 Run deferred builds, tests, format checks, and OpenSpec validation after authorization.
- [ ] 3.2 Update feature trackers at shipment and archive after execution.
