# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `caret_blink` | Open a Horizontal Type session; wait one interval; type "a" | Interval 750 ms; shown on open, hidden after the interval, shown right after the key | `tst_type_tools::caretBlinks` |
| `caret_placement` | Click with Horizontal Type; type "some text" | Placement point at the click, no guide; then one guide from the click along its baseline to the caret | `tst_type_tools::caretPlacementAndBaseline` |
| `caret_baselines_engine` | Baselines of horizontal "ab\r\rc" and vertical "ab" | Two horizontal segments (none for the blank line), two leadings apart, each ending at the line's last stop; one vertical segment on the column axis | `pictura-render::baselines_follow_the_layout` |
| Looks like CS6's I-beam | Click with the Type tool on a real display; watch it; type | A thin I-beam with a square at the baseline blinks; typing shows an underline guide; the caret blinks at the end | manual: blink appearance and on-screen pixel look need a real display |
