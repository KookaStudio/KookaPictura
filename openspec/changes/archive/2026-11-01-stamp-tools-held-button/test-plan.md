# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `clone_hold_airbrush_off` | Hold a low-flow Clone Stamp still, Airbrush off | Pixels unchanged | `tst_stamp_hold::cloneHoldAirbrushOff` |
| `clone_hold_airbrush_on` | Hold it still, Airbrush on, release | Pixels build; one history state | `tst_stamp_hold::cloneHoldAirbrushOn`, `pictura-paint::hold_builds_to_opacity_cap` |
| `clone_hold_airbrush_toggle` | Click the toggle | Option on; survives JSON | `tst_stamp_hold::airbrushTogglesPersist` |
| `pattern_hold_airbrush_off` | Hold a low-flow Pattern Stamp still, Airbrush off | Pixels unchanged | `tst_stamp_hold::patternHoldAirbrushOff` |
| `pattern_hold_airbrush_on` | Hold it still, Airbrush on, release | Pixels build; one history state | `tst_stamp_hold::patternHoldAirbrushOn` |
| `pattern_hold_airbrush_toggle` | Click the toggle | Option on; survives JSON | `tst_stamp_hold::airbrushTogglesPersist` |
| `history_hold_airbrush_off` | Hold a low-flow History Brush still, Airbrush off | Pixels unchanged | `tst_stamp_hold::historyHoldAirbrushOff` |
| `history_hold_airbrush_on` | Hold it still, Airbrush on, release | Pixels build; one history state | `tst_stamp_hold::historyHoldAirbrushOn` |
| `history_hold_airbrush_toggle` | Click the toggle | Option on; survives JSON | `tst_stamp_hold::airbrushTogglesPersist` |
| `art_history_hold` | Hold an Art History Brush still, release | More pixels change; one history state | `tst_stamp_hold::artHistoryPaintsWhileHeld`, `pictura-paint::hold_paints_art_history` |
| `stamp_hold_stops` | End a held stroke by release, tool switch, cancel | Pixels stop changing | `tst_stamp_hold::holdStopsOnRelease`, `tst_stamp_hold::holdStopsOnToolSwitch`, `tst_stamp_hold::holdStopsOnSessionCancel` |
| `stamp_switch_commits` | Switch tool during a stroke | One committed history state | `tst_stamp_hold::toolSwitchCommitsStroke` |
| Spacing after a hold | Move on after holding | Path spacing unchanged | `pictura-paint::repeat_last_keeps_path_spacing` |
| Flow 0 | Hold at Flow 0 | No change | `pictura-paint::hold_paints_nothing_at_flow_zero` |
| Raster equivalence | Hold on banded and serial strokes | Same bytes | `pictura-paint::hold_matches_serial_raster` |
| App focus lost mid-press | Hold with Airbrush on, Alt-Tab away | Painting stops at once | manual: offscreen Qt has no real application focus change |
| Feel at 20 dabs/s | Hold at 10% Flow with a soft tip | Build-up looks smooth | manual: visual judgement; the CS6 rate is undocumented |
