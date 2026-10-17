# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `type_session_move` | Type "Move me"; read the zone away from and over the text; drag from outside by (30, 20); commit | Move zone outside, not over the text; placement and guide moved by (30, 20); no history mid-drag; layer 30 px right and 20 px lower than unmoved | `tst_type_tools::dragOutsideMovesTheText` |
| `type_session_click_commits` | Type, then click away without dragging | One "Horizontal Type" state | `tst_type_tools::typeKeysAndCancel` |
| Move cursor on screen | On a real display, type, then hover away from the text and drag | The Move arrow shows outside the text, the I-beam over it; the text and its guides follow the drag | manual: the offscreen platform has no real pointer, so the cursor shape is checked through its zone id above |
