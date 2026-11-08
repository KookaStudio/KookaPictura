# Test Plan — maximum-minimum-preserve

One row per scenario in the change's spec deltas. Rows name a test this change
creates unless marked `(existing)`.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Maximum grows bright regions and Minimum shrinks them | Apply Maximum and Minimum at the same non-zero radius to a white-on-black mask | The white region grows under Maximum and shrinks under Minimum | `pictura-filters::maximum_brightens_and_minimum_darkens_known_values` (existing) |
| Radius zero is a no-op | Apply Maximum and Minimum with radius 0 | The buffer is bit-exactly unchanged | `pictura-filters::morphology_radius_zero_is_noop` (existing) |
| Radius above 500 clamps | Apply Maximum with radius 600 and with radius 500 to the same buffer | The two outputs are bit-identical and no error is returned | `pictura-filters::morphology_radius_reaches_five_hundred_and_clamps` |
| Matches ImageMagick | Apply Maximum and Minimum with a given radius and compare with ImageMagick `Square:N` | No sample differs by more than 1 | `pictura-filters::maximum_matches_imagemagick` (existing) |
| `morph_squareness` | Apply Maximum and Minimum with `Squareness` at radius 1 to a 3×3 buffer whose only bright sample is the inspected pixel's diagonal neighbour | The diagonal sample is inside the square footprint: Maximum takes it and Minimum ignores it | `pictura-filters::maximum_squareness_keeps_the_square_footprint` |
| `morph_roundness` | Apply Maximum and Minimum with `Roundness` at radius 1 and radius 2 to the same buffer | Radius 1 ignores the diagonal; radius 2 includes the `(1, 1)` offset | `pictura-filters::maximum_roundness_uses_the_disk_footprint` |
| `morph_roundness_alpha` | Apply Maximum and Minimum with `Roundness` to a four-channel buffer whose alpha carries the diagonal pattern | The alpha plane follows the disk rule, matching the colour plane | `pictura-filters::roundness_moves_the_alpha_plane` |
| `morph_radius_500` | Maximum at radius 150 vs 100 with a bright sample 120 px away; Maximum at radius 600 vs 500 with a bright sample 520 px away | Radius 150 takes the 120 px sample; radius 600 equals radius 500 and leaves the 520 px sample outside | `pictura-filters::morphology_radius_reaches_five_hundred_and_clamps` |
| `morph_preserve_slots` | Map `maximum` and `minimum` with `[]`, `[r, 0]`, `[r, 1]`, and a three-value list | Empty and `0` build `Squareness`, `1` builds `Roundness`, the three-value list is refused | `pictura-filters::morphology_preserve_selects_the_footprint` |
| `morph_dialog_defaults` | Open the Maximum and Minimum dialogs from the descriptor | Values are `2, 0`; Radius ranges `1..=500`; the Preserve menu lists Squareness and Roundness with Squareness selected; the warning is hidden | `tst_maximum_filter::dialogFollowsCs6` |
| `morph_dialog_warning` | Set Radius to 150, then to 100, and read the warning label | The warning is visible above 100 and hidden at 100 | `tst_maximum_filter::warningAppearsAboveOneHundred` |
| `morph_dialog_commit` | Choose Roundness, accept, read the history and last params, then invoke Last Filter | The canvas changes, one `Filter: Maximum` state is recorded with Preserve slot 1, and Last Filter re-applies it | `tst_maximum_filter::roundnessCommitsAndRepeats` |
| Visual comparison with a modern Photoshop capture | Apply Maximum and Minimum at the same radius in both applications and compare the round and square results | Same direction and footprint family; no exact pixel match claimed | manual: Adobe's disk kernel is closed and no oracle exists, so the spec records the formula as an approximation |
