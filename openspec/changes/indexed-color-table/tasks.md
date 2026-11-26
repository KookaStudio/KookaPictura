## 1. Indexed engine and persistence

- [x] 1.1 Preserve opacity and all distinct forced colors; reject capacity overflow and invalid Exact requests atomically; support index-preserving table replacement. Add `pictura-render::forced_indexed_transparency_capacity_boundary`, `pictura-render::invalid_exact_request_is_atomic`, and `pictura-render::exact_indexed_conversion_fits_capacity`.
- [x] 1.2 Store and read indexed count and transparency resources and keep PSD/PSB output in Indexed mode. Add `pictura-codec::indexed_metadata_round_trips_psd_and_psb`.

## 2. Qt and bridge

- [x] 2.1 Add document conversion controls, recompute Exact availability and apply options during preview and commit. Add `tst_image_mode::exactFallsBackWhenOptionsExceedCapacity` and `tst_image_mode::exactRemainsAvailableAtCapacity`.
- [x] 2.2 Enable and implement Color Table, including entry and range editing, ACT file IO and undo. Add `tst_image_mode::colorTableInterpolatesRange`, `tst_image_mode::colorTableActRoundTrip`, `tst_image_mode::colorTableRejectsShortAct`, and `tst_image_mode::colorTableUnchangedAcceptance`.

## 3. Scenario coverage

- [x] 3.1 Write engine and Qt tests from the test plan.
- [ ] 3.2 Run deferred build, tests and OpenSpec validation.
