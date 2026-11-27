# Test plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| slm_discovery | Open marquee flyout, Alt-click slot, exercise Shift+M and plain M from each line tool with both switch preferences and inspect shortcuts. | Both line tools reachable, unassigned, Shift+M cycles rectangle/ellipse; keyboard cycling from either line tool selects rectangle, and plain M selects rectangle when Shift is required. | `tst_single_line_marquee::discovery` |
| slm_shortcut_fallback | Select each line tool and press the M-cycle gesture under both preferences; also press plain M with Shift required. | Rectangular Marquee becomes active in each case. | `tst_single_line_marquee::discovery` |
| slm_row | Click (3,2) on 8×6 with row tool and zero feather. | Exactly row 2 is 255. | `tst_single_line_marquee::row` |
| slm_column | Click (3,2) on 8×6 with column tool and zero feather. | Exactly column 3 is 255. | `tst_single_line_marquee::column` |
| slm_one_pixel | Click a 1×1 document with each tool. | The only pixel is 255. | `tst_single_line_marquee::onePixel` |
| slm_history | Click inside prior selection in New mode; undo, redo. | Band replaces prior mask; undo and redo restore each mask. | `tst_single_line_marquee::history` |
| slm_modes | For both tools combine overlapping bands using each button and modifier. | Mask equals the requested combine result. | `tst_single_line_marquee::modes` |
| slm_feather | Select each orientation with feather, including a combine case. | Partial mask coverage is produced before combination. | `tst_single_line_marquee::feather` |
| slm_feather_full | Feather each orientation on a document fully covered by its band. | Every pixel remains 255. | `tst_single_line_marquee::featherFull` |
| slm_feather_warning | Create a nonempty feathered band with all pixels at or below 50%; dismiss the warning, undo and redo. | Warning is shown, mask survives dismissal, and history restores both masks. | `tst_single_line_marquee::featherWarning` |
| slm_options | Activate each tool and inspect options. | Modes, Feather and Refine Edge present; Style and Anti-alias absent. | `tst_single_line_marquee::options` |
| slm_refine | Create a selection with each tool and activate Refine Edge from its options bar. | The shared Refine Edge action opens. | `tst_single_line_marquee::refineEdge` |
| slm_move | Drag inside an existing selection with each tool. | Existing selection moves. | `tst_single_line_marquee::moveSelection` |
| slm_context | Open canvas context menu with each tool and a selection. | Selection menu actions are available. | `tst_single_line_marquee::contextMenu` |
