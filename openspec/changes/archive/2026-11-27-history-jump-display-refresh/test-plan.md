# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `eh_jump_strokes_display` | Record three brush strokes, then jump to the first stroke state | Only the first stroke is visible in the canvas and `sample_argb` | `tst_history_options::jumpBackAcrossStrokesRefreshesTheDisplay` |
| `eh_jump_anchor_display` | After a non-linear branch capture of a fourth stroke, jump to the first stroke state | The fourth stroke is gone from the canvas and `sample_argb` | `tst_history_options::jumpBackAcrossStrokesRefreshesTheDisplay` |
