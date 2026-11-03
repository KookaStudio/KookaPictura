# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `crr_content_agrees` | Test opaque, empty, one-pixel, alpha-less, zero-width and missing layers | Matches the bounds; Align and Distribute enable as before | `pictura-render::the_content_test_agrees_with_content_bounds` |
| `crr_coverage_agrees` | Test empty, one-pixel, full, short and banded planes | Matches the bounds | `pictura-render::plane_coverage_test_matches_its_bounds` |
| `crr_scan_once` | Test a plane and its copy three times, write, test again | One scan, then a second after the write | `pictura-render::an_unchanged_plane_is_scanned_once` |
| `crr_no_materialize` | Test a plane with a diverged band | No materialization | `pictura-render::a_diverged_plane_is_checked_without_materializing` |
| `crr_selection_agrees` | Test five selections for a lift | Matches `lift_coverage` | `pictura-app::the_selection_test_agrees_with_the_lift_coverage` |
| `crr_switch_no_cancel` | Cancel with nothing live | No `sessionCancelled` | `tst_document_session::cancellingNothingReportsNoCancel` |
| `crr_switch_cancels_live` | Cancel mid-stroke | One `sessionCancelled` | `tst_document_session::cancellingAStrokeReportsTheCancel` |
| `crr_whole_region` | Change a layer everywhere, refresh the whole document as a region | Equals a rebuild; level 0 owns its plane; frame unchanged | `pictura-app::a_whole_document_region_refresh_installs_its_frames` |
| `crr_large_manual` | On a real display, new 16507×16196 document plus an empty layer: switch tools, drag a Marquee, Paint Bucket fill | No visible stall on switches or the Marquee | manual: offscreen Qt has no real pointer or compositor timing |
