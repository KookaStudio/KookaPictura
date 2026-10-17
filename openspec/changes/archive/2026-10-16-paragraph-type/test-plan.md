# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `para_drag_box` | Drag 120 × 80 with Horizontal Type; type a long sentence; Ctrl+Enter; click the layer again | Box shown; preview wraps within the width; one "Horizontal Type" state; layer box = dragged box; rect inside it; the reopened session has the same box | `tst_paragraph_type::dragMakesParagraphType` |
| `para_wrap_engine` | Narrow box horizontal text; a word wider than the box; vertical text in a short box | Breaks after spaces; no line over the width; trailing spaces kept; long word broken; columns of the box height; stops = UTF-16 len + 1 | `pictura-render::paragraph_wraps_words_and_long_words` |
| `para_alt_click` | Alt-click and Cancel; Alt-click and enter 90 × 40 | No session; then a session with a 90 × 40 box at the click | `tst_paragraph_type::altClickAsksForTheBoxSize` |
| `para_resize_overflow` | Long text in a short box; drag the bottom-right handle out | Overflow reported; the box grows, the text reflows, the overflow clears; no history until commit | `tst_paragraph_type::handlesResizeAndOverflow` |
| `para_overflow_engine` | Four lines' text in a two-line box; then a four-line box | Ink only in the first two line bands; overflow true; then four lines and no overflow | `pictura-render::paragraph_overflow_is_hidden_and_reported` |
| `para_psd_roundtrip` | Author, encode, and decode a paragraph spec and a point spec, also merged over existing EngineData | Paragraph decodes with the same box and ShapeType 1; point decodes with no box | `pictura-codec::paragraph_box_round_trips` |
| `para_convert` | Type > Convert To Paragraph Text on a point layer, then Convert To Point Text | Each enabled only for its kind; one state each; box present, then absent; same text | `tst_paragraph_type::convertCommands` |
| `para_convert_engine` | Two-line point → paragraph → point; wrapped, overflowing paragraph → point | Identical pixels; text and origin restored; returns at the wraps, overflow dropped | `pictura-render::conversion_keeps_glyphs_in_place` |
| Box look and handle feel | On a real display, drag a box, type past its bottom, drag handles | Dashed box, eight square handles, a `+` in the bottom-right handle on overflow; handles track the pointer | manual: on-screen rendering and pointer feel need a real display |
| Opens in Photoshop CS6 | Save a paragraph layer to PSD and open it in Photoshop CS6 | It opens as live paragraph type in the same box | manual: needs a licensed Photoshop CS6; no automated oracle reads ShapeType |
