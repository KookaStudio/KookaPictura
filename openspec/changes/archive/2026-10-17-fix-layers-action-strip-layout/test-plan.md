# Test Plan

Covers the `ui/layers-panel` delta. The scenario is a pure widget-geometry check,
so it is automated with Qt Test.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lpc_strip_geometry` | Construct a standalone `LayersPanel`; choose a narrow width of `minimumSizeHint().width() + 80` and a wide width of narrow + 240; show it, wait 50 ms, assert the requested width took effect, and record the seven buttons' geometry; resize wide, activate the layout, wait 50 ms, assert the requested width took effect, and record again | Before and after the resize every adjacent gap is 2 px, each button keeps its width, and the left and right margins stay equal within 1 px | `tst_layers_panel_fixes::actionStripStaysFixedAndCentred` |

The requirement's unchanged-contents clause (button order, object names, icons,
tooltips, click behaviour, and drop targets) is guarded by task 2.2's `git diff`
review and `tst_layers_panel`, `tst_layers_panel_fixes`, `tst_panels_parity`, and
`tst_icon_assets`, not by the geometry case.
