# Tasks

## 1. Banded level-0 source

- [x] 1.1 `Planes` accepts a planar `Plane<u8>` and serves channel rows through
      `Plane::row`; `Planes::rows` returns one line's four channels.
- [x] 1.2 `sample_from_planes`, `shrink_from_planes`, `crop` and the premultiply
      helper read level-0 rows instead of channel slices.
- [x] 1.3 `planes_of` builds the planar backing.
- [x] 1.4 Tests `a_banded_level0_builds_the_same_levels` and
      `a_banded_level0_updates_and_crops`, both asserting the banded source is
      never materialized.

## 2. Shared level 0

- [x] 2.1 `level0_buffer` fractures the composite and `level0_from_buffer` the
      frame instead of copying a shared frame.
- [x] 2.2 `refresh_level0_region`'s whole-frame branch fractures instead of
      copying.
- [x] 2.3 Tests `a_whole_document_restore_shares_the_composite_plane`,
      `a_region_patch_forks_only_its_bands`, and the sharing assertion in
      `level0_carries_the_document_profile_conversion`.

## 3. Unchanged composite at a segment break

- [x] 3.1 `history/planes.rs` gains `composite_unchanged`, a band-parallel
      early-exit comparison.
- [x] 3.2 `capture_live_with` keeps the current state's composite plane when the
      picture did not change.
- [x] 3.3 Tests `a_segment_break_keeps_an_unchanged_composite`,
      `a_segment_break_keeps_a_changed_composite`.

## 4. Solo mask source

- [x] 4.1 `mask_display_plane` fills its planes as parallel row work from a
      zeroed buffer.
- [x] 4.2 Test `mask_display_plane_matches_a_serial_build`.

## 5. Verification

- [x] 5.1 `structural_restore_profile` records the resize and reorder restore
      costs; `solo_mask_profile` compares one core with all cores. The design's
      measurement table holds the results.
- [x] 5.2 Update `docs/dev/STATE.md` with the change and its numbers.
