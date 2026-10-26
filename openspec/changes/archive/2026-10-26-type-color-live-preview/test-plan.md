# Test plan

The Text Color picker is modal, so each check drives the real dialog from a
timer: it types a hex value, inspects the layer while the picker is still open,
then accepts or cancels.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `tclp_picker_live` | Open `ColorPickerDialog::getColor` with a live callback, type `0000ff`, then accept; repeat and cancel | The callback hears blue before the dialog closes; on Cancel its last call is the opening color | `tst_type_color_preview::pickerReportsLiveChanges` |
| `tclp_bar_live` | Select a red type layer with the Type tool, click the bar swatch, type `0000ff` | Layer spec and pixels are blue while the picker is open; history index unchanged | `tst_type_color_preview::barPreviewsThenOkCommitsOnce`, `tst_type_tools::typeBarSizeAlignAndColor` |
| `tclp_bar_ok` | As above, then OK | Layer stays blue; one "Edit Type Layer" state; Undo returns red | `tst_type_color_preview::barPreviewsThenOkCommitsOnce` |
| `tclp_bar_cancel` | As above, then Cancel | Layer is red again, pixels included; history unchanged | `tst_type_color_preview::barCancelRestores` |
| `tclp_session` | Type "HH" without committing, open the bar picker, type `0000ff`, Cancel | Session text color is blue while open, red after Cancel; no state | `tst_type_color_preview::sessionPreviewsAndRestores` |
| `tclp_panel` | Select a red type layer, click the Character panel swatch, type `0000ff`; OK in one run, Cancel in another | Blue with no state while open; OK records one state; Cancel restores red with none | `tst_type_color_preview::characterPanelPreviewsAndCommits`, `tst_type_color_preview::characterPanelCancelRestores` |
| System picker branch | Preferences > General > Color Picker set to System, repeat the bar steps | Same live behavior through `QColorDialog` | manual: the native/Qt dialog cannot be driven by a field edit in the offscreen suite |
