# Test plan — deterministic-icc-serialization

Every spec scenario maps to a runnable check. Rust tests are std `#[test]` run by
`cargo nextest`; the oracle row self-skips when PIL is absent.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `icc_serial_repeat` | Call `Profile::adobe_rgb().to_icc()` twice and compare | The two byte vectors are equal | `pictura-color::serialized_profiles_are_byte_identical` |
| `icc_serial_zero` | Serialize a built-in profile and read header bytes 24..35 | All twelve bytes are zero | `pictura-color::serialized_profiles_have_a_zeroed_creation_date` |
| `icc_serial_reload` | Pass `to_icc()` bytes to `Profile::from_icc` | It loads and has the same color-space signature | `pictura-color::icc_round_trips` (existing) |
| `icc_serial_external` | Assign Adobe RGB, write the PSD, extract resource 1039, open it with PIL | PIL opens the profile and reports its description | `pictura-codec::assigned_profile_round_trips_through_saved_psd` (oracle, self-skips without PIL) |

## Notes

- `icc_serial_zero` is the deterministic regression guard: it fails immediately
  when the zeroing is removed. `icc_serial_repeat` is complementary — two
  back-to-back calls normally land in the same wall-clock second, so it passes
  even on unfixed code and cannot catch the regression on its own.
- The residual date-flaky `to_icc()` comparisons in `pictura-codec`
  (`src/icc.rs` `assign_frames_a_1039_block_for_a_non_srgb_profile` and
  `assign_preserves_an_unparsed_resource_tail`, and
  `tests/profile_assignment_oracle.rs` `assigned_profile_round_trips_through_saved_psd`)
  are covered by task 4.1, which re-runs them unchanged.
- The ImageMagick differential oracle
  (`pictura-color::convert_srgb_to_adobe_matches_lcms2`, task 4.2) also reads
  `to_icc()` bytes with a zeroed date; it self-skips when `magick` is absent, so
  PIL is the external-read coverage of record in that case.
