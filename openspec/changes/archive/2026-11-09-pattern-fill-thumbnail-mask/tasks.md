# Tasks

## 1. Tests first

- [x] 1.1 `pictura-codec::patterns::tests::photoshop_slot_layout_decodes_written_color_planes` — a 24-colour-slot pattern decodes its three written RGB planes and trailing alpha.
- [x] 1.2 `pictura-codec::patterns::tests::unused_colour_slots_are_not_decoded` — a written middle colour slot with a 16-bit plane is ignored.
- [x] 1.3 `pictura-codec::patterns::tests::a_colour_slot_count_above_photoshop_layout_is_skipped` — a 25-slot pattern is rejected before reading channels.
- [x] 1.4 `pictura-codec::patterns::tests::grayscale_pattern_ignores_an_extra_colour_slot` — an extra grayscale colour slot is ignored, replacing the old strict rejection.
- [x] 1.5 `pictura-codec::pattern_fill_masked_fixture_decodes_pattern_and_mask` — the generated Photoshop-layout fixture decodes its 24-slot pattern and painted mask.
- [x] 1.6 `tst_layers_snapshot::patternFillThumbnailAndMask` — the opened fixture's fill row shows the tiled pattern and a non-null mask thumbnail.

## 2. Decoder

- [x] 2.1 Read `num_channels` as Photoshop's colour-slot count: accept 1–24 slots (at least the mode's plane count), not only the mode's plane count.
- [x] 2.2 Decode only the leading colour slots and the two alpha slots; skip the middle slots without decompressing their planes.
- [x] 2.3 Keep the alpha rule (last written trailing slot) and the existing 8-bit, rectangle, and pixel-cap guards.

## 3. Fixture and docs

- [x] 3.1 Generate `crates/pictura-codec/tests/fixtures/pattern_fill_masked.psd` from `scripts/generate-fixtures.py` and document it in the fixture README.
- [x] 3.2 Register the fixture in `crates/pictura-codec/tests/oracle/support.rs` and add the oracle test.
- [x] 3.3 Update the fixture README to describe the Photoshop 24-slot pattern layout.

## 4. Finish

- [x] 4.1 Run `cargo nextest run -p pictura-codec -p pictura-app`, `tst_layers_snapshot`, and `scripts/check-test-plan.py`.
- [x] 4.2 Run `scripts/verify-fast.sh`.
- [x] 4.3 Archive the change.
