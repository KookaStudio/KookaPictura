# Test plan

This plan covers `raw-open-route` (#400): the new capabilities
`codec/raw-develop` and `document/raw-open`, and the requirement added to
`codec/pictura-raw`.

**Fixtures.** Codec tests build DNGs and CR2s with the existing synthetic
builders (`tests/camera_raw/dng/builder.rs`, `tests/camera_raw/raw/cr2_builder.rs`). The app's Rust
tests and the Qt suite write a minimal uncompressed 16-bit RGGB DNG with their
own small in-test writer (no binary fixture is committed); its `ColorMatrix1`
is XYZ-D50-to-linear-ProPhoto at D50, so the colour stage is close to the
identity and expected values are easy to state. The sRGB encoding of 0.18
(0.4613) is the published sRGB transfer function, written into the test.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raw_source_decodes` | Decode a 16 × 12 RGGB DNG with ColorMatrix1, AsShotNeutral, default crop | Camera RGB 16 × 12; crop, orientation equal the file's; as-shot = `as_shot_transform` | `pictura-codec::dng_decodes_to_raw_source` |
| `raw_source_refuses_colour` | Decode a DNG without ColorMatrix1 (UniqueCameraModel "Test Cam") and a synthetic CR2 | Both `Unsupported`; DNG message has "camera colour" and "Test Cam" | `pictura-codec::colourless_files_are_refused_before_demosaic` |
| `raw_source_refuses_non_raw` | Decode a TIFF without DNGVersion and an empty buffer | Both `Unsupported` | `pictura-codec::non_raw_bytes_are_refused` |
| `raw_develop_neutral_grey` | ProPhoto camera, uniform 0.18, as-shot | Equal channels within 64; 0.4613 ± 0.01 of full scale | `pictura-codec::neutral_grey_develops_to_srgb_grey` |
| `raw_develop_white_balance` | Same grey at 3000 K and 9000 K | R/B at 9000 K > R/B at 3000 K | `pictura-codec::warmer_temperature_is_redder` |
| `raw_develop_baseline_exposure` | 0.1 grey at BE 0 and 1; 0.8 grey at BE 1 | Linear 0.2 ± 0.01; white | `pictura-codec::baseline_exposure_scales_linear_light` |
| `raw_develop_default_crop` | 12 × 10 with crop (2, 1, 8, 6), one marked pixel at (2, 1) | 8 × 6; top-left is the marked pixel | `pictura-codec::default_crop_is_applied` |
| `raw_develop_orientation` | 4 × 2 with a marked corner, orientations 1–8 | Sizes swap for 5–8; marked pixel per TIFF table | `pictura-codec::orientation_rotates_and_mirrors` |
| `raw_preview_downscale` | 40 × 20 checker of 2 × 2 blocks to 10; 6 × 4 to 10 | 10 × 5 all 2000 ± 1; unchanged | `pictura-codec::downscale_averages_and_bounds` |
| `raw_xmp_embedded` | DNG with tag 700 `crs:Temperature="4300"`, `crs:Exposure2012="+0.50"` | Packet returned; 4300 and 0.5 | `pictura-codec::embedded_crs_settings_are_read` |
| `raw_xmp_absent` | DNG without tag 700; PNG bytes | None for both | `pictura-codec::missing_xmp_reads_none` |
| `pictura_raw_rgb16_default_noop` | Random 16-bit image, default settings | Unchanged | `pictura-adjust::rgb16_default_settings_are_noop` |
| `pictura_raw_rgb16_matches_8bit` | 8-bit × 257 image; exposure +0.5, contrast 20, saturation −30 at both depths | 16-bit / 257 within 1 of 8-bit | `pictura-adjust::rgb16_matches_8bit_on_8bit_values` |
| `pictura_raw_rgb16_refuses` | Buffer one sample short; 0 × 0 | Errors | `pictura-adjust::rgb16_refuses_malformed_buffers` |
| `raw_open_sniff` | `raw_sniff_path` on a DNG named `shot.tif`, a plain TIFF, a missing path | Only the DNG is true | `pictura-app::raw_sniff_detects_by_signature` |
| `raw_open_filter` | Read `openFileFilters()` | "Camera Raw (*.DNG *.CR2)" present; All Formats has both | `tst_raw_open::openFiltersListCameraRaw` |
| `raw_open_dialog_ok` | Open the fixture interactively; driver records document count, accepts | Dialog seen with no new document; one 16-bit RGB 16 × 12 doc, one layer, one state | `tst_raw_open::dialogOkOpensSixteenBit` |
| `raw_open_dialog_cancel` | Open interactively; driver cancels | Cancelled; document count unchanged | `tst_raw_open::dialogCancelOpensNothing` |
| `raw_open_noninteractive` | `openDocumentAtPathOutcome(path, false)` with a driver that counts | No dialog; 16-bit doc opens | `tst_raw_open::openWithoutDialog` |
| `raw_open_develop_matches_codec` | App develop at seed vs `develop_raw` at as-shot | Samples equal | `pictura-app::develop_matches_codec_at_seed` |
| `raw_open_seed_as_shot` | Seed for a DNG without XMP | Temp/Tint = rounded as-shot | `pictura-app::seed_uses_as_shot_white_balance` |
| `raw_open_seed_sidecar` | `shot.xmp` (3200, +1.00) beside a DNG embedding 4300 | Temp 3200, Exposure 1.0 | `pictura-app::sidecar_overrides_seed` |
| `raw_open_seed_embedded` | DNG embedding 4300, no sidecar | Temp 4300 | `pictura-app::embedded_xmp_seeds_without_sidecar` |
| `raw_open_absolute_sliders` | Raw-mode dialog seeded at 5500 K; layer-mode dialog | Raw: Temp 2000–50000 at 5500, Tint −150…150; layer: ±100 | `tst_raw_open::rawDialogShowsAbsoluteSliders` |
| `raw_open_reciprocal_slider` | Reciprocal 2000–50000 slider, value at the track's middle | 3500–4500 K | `tst_raw_open::reciprocalSliderSpreadsDaylight` |
| `raw_open_temperature_preview` | Preview at 3000 and 9000 K | Mean R/B larger at 9000 K | `pictura-app::preview_follows_temperature` |
| `raw_open_unsupported` | Open a DNG without ColorMatrix1 non-interactively | Failed; no document; status names the file and "camera colour" | `tst_raw_open::unsupportedColourFailsClearly` |

Manual (reviewer): open a real DNG (for example the Leica M9 samples in
`PICTURA_DNG_CORPUS`) with File ▸ Open, drag Temperature and Exposure, press
OK, and check the document looks like the camera's rendering; open a real CR2
and check the refusal message.
