# Test plan — color-replacement-modes

Every scenario in the delta spec has one row. Coverage is the last column and
names the automated test that proves it; the new tests are written in the
implementation step before the code they exercise.

## tools/color-replacement-tool

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Color keeps the shading | Replace a grey ramp with red under Color | Both ends turn red; the darker end stays darker | `pictura-paint::color_mode_keeps_the_shading` |
| Contiguous stops at a gap | Dab across two matching stripes separated by a non-matching one | Contiguous skips the far stripe; Discontiguous replaces it | `pictura-paint::contiguous_stops_at_a_gap_and_discontiguous_jumps_it` |
| Once keeps the first sample | Start a Once stroke on green and move onto red | The green is replaced; the red is unchanged | `pictura-paint::once_sampling_keeps_the_first_colour` |
| `cr_behind_clear` | Dab Behind over opaque pixels, then Clear over matched opaque pixels | Behind leaves the pixels byte-for-byte unchanged; Clear sets alpha to zero | `pictura-paint::behind_leaves_opaque_pixels_and_clear_erases_them` |
| `cr_mode_menu` | Open the Color Replacement Mode menu on an RGB document | 25 entries from Normal through Luminosity, Color selected, and every index round-trips | `pictura-paint::every_cs6_paint_mode_round_trips`, `tst_color_replacement::modeMenuListsTheCs6PaintModes` |
| `cr_normal_multiply` | Blend a mid-grey pixel with red under Normal and Multiply | Normal gives the foreground; Multiply gives the per-channel product | `pictura-paint::normal_and_multiply_apply_the_blend` |
| `cr_32bit_subset` | Open the Mode menu on a 32-bit RGB document, then convert it to 8-bit | 32-bit offers the 11 allowed modes and no Color Burn; 8-bit restores all 25 | `pictura-paint::thirty_two_bit_documents_offer_the_subset`, `tst_color_replacement::thirtyTwoBitDocumentFiltersTheModeList` |
| `cr_bitmap_unavailable` | Open a Bitmap document, inspect the B flyout, and try to select Color Replacement | The flyout row is disabled, the active tool stays Move, and the bridge refuses a begin | `pictura-app::color_replacement_is_unavailable_for_flat_modes`, `tst_color_replacement::bitmapDocumentDisablesTheTool` |
| `cr_indexed_unavailable` | Activate Color Replacement on RGB, convert to Indexed, and refresh | The tool switches to Move and the flyout row is disabled | `tst_color_replacement::indexedDocumentDisablesTheTool` |
| `cr_multichannel_unavailable` | Evaluate the shared availability predicate for `multichannel` | Color Replacement is refused | `pictura-app::color_replacement_is_unavailable_for_flat_modes`, `tst_color_replacement::multichannelModeIsUnavailable` |
| `cr_rgb_available` | Open an RGB document and activate Color Replacement | The flyout row is enabled and the tool becomes active | `tst_color_replacement::rgbDocumentKeepsTheToolAvailable` |
| Repaint a field | Drag a green brush across a blue field beside a yellow one | One "Color Replacement Tool" state; blue turns green; yellow is unchanged | `tst_color_replacement::repaintAField` |
