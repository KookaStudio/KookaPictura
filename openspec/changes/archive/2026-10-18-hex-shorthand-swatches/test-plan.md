# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cp_hex_prefix` | Clear the picker's hex field and type `#aabbcc`, then `0xAABBCC` | The picker's colour is `#aabbcc` after each entry | `tst_color_picker::hexAcceptsPrefixes` |
| `cp_hex_shorthand` | Type `#123` and commit the field | The colour becomes `#112233` and the field shows `112233` | `tst_color_picker::hexShorthandExpandsOnCommit` |
| `cp_hex_invalid` | Type `#12` and commit | The colour is unchanged and the field returns to the current colour's hex | `tst_color_picker::invalidHexLeavesColor` |
| `cp_add_swatch` | Open the picker with a Swatches grid, click Add to Swatches, confirm the prompt | The prompt defaults to `Swatch 1`; the current colour is appended under the entered name and the picker stays open | `tst_color_picker::addToSwatchesAppendsCurrentColor` |
| `cp_add_swatch_cancel` | Click Add to Swatches and cancel the prompt | No swatch is appended | `tst_color_picker::addToSwatchesCancelsCleanly` |
| `cp_add_swatch_disabled` | Construct the picker without a Swatches grid | Add to Swatches is disabled | `tst_color_picker::addToSwatchesIsDisabledWithoutPanel` |
| `sw_add_from_picker` | The picker adds a colour to the panel | The grid contains the colour under the entered name | `tst_color_picker::addToSwatchesAppendsCurrentColor` |
| `sw_generated_name` | Invoke New twice on a fresh panel | The added swatches are named `Swatch 1` and `Swatch 2` | `tst_swatches_panel::newSwatchUsesGeneratedName` |
