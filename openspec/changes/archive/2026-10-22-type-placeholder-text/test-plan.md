# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tph_box` | Drag a box with the Horizontal Type tool | The session text is `Lorem Ipsum Pictura`, all selected | `tst_type_placeholder::dragFillsBox` |
| `tph_click` | Click with the Horizontal Type tool | The session text is `Lorem Ipsum Pictura`, all selected | `tst_type_placeholder::clickFillsPointType` |
| `tph_typing` | Drag a box, type `Hello`, commit | The new layer's text is `Hello` | `tst_type_placeholder::typingReplacesPlaceholder` |
| `tph_commit` | Click, then commit without typing | A type layer reading `Lorem Ipsum Pictura` is added | `tst_type_placeholder::untouchedPlaceholderCommits` |
| `tph_exempt` | Click with Horizontal Type Mask; click an existing type layer with Horizontal Type | The mask session is empty; the reopened session holds the layer's text with no selection | `tst_type_placeholder::maskAndReopenAreExempt` |
| `tph_pref_page` | Dispatch `Edit > Preferences > Type` | The Type page shows with the checkbox checked | `tst_type_placeholder::typePreferencePage` |
| `tph_pref_off` | Clear the checkbox, then click with Horizontal Type | The session starts empty | `tst_type_placeholder::preferenceOffStartsEmpty` |
| `tph_pref_saved` | Clear the checkbox and load the saved session | The saved session has the placeholder preference off | `tst_type_placeholder::preferenceIsSaved` |
| `wt_icon` | Inspect the Horizontal Type options page | The Warp Text button's icon is non-null and differs from `options.warp` | `tst_warp_text::optionsButtonHasWarpTextIcon` |
| Placeholder highlight on screen | Drag a box with the Type tool on a real desktop | "Lorem Ipsum Pictura" shows with the selection highlight, as in the reference screenshot | manual: offscreen tests check the selection range, not how the highlight looks |
| Warp Text icon artwork | Look at the type options bar in light and dark themes | The button shows a T standing on an arc, legible in both themes | manual: the test checks the icon is distinct, not that it reads well |
