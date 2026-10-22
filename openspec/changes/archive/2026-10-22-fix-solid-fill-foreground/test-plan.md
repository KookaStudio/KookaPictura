# Test Plan

Covers the `imaging/adjustment-layer-rendering` delta. The scenarios are
panel-menu and Layer-menu checks against the real `PicturaMainWindow`, so they
are automated with Qt Test. The colour is read back from the composited
canvas, so the assertion covers the whole path: the action, the bridge, the
`SoCo` block, and the render.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `scf_panel_foreground` | New document, find the shared colour state on the window, set its foreground to a non-black colour, bind the panel's view, then trigger the `Solid Color…` action in the panel's New Fill / Adjustment strip menu | A solid-color fill layer is added and the canvas pixel under it is the foreground colour, not black | `tst_layers_panel_fixes::solidFillUsesForeground` |
| `scf_menu_foreground` | New document, set the shared colour state's foreground to a non-black colour, then dispatch `LayerNewFillSolidColor` through the command registry | A solid-color fill layer is added and the canvas pixel under it is the foreground colour, the same colour the panel path uses | `tst_layer_commands::solidFillCommandUsesForeground` |
| `scf_default_black` | New document, set the shared colour state's foreground to black and its background to white, then trigger the panel's `Solid Color…` action | A solid-color fill layer is added and the canvas pixel under it is opaque black, not the background white | `tst_layers_panel_fixes::solidFillUsesForegroundNotBackground` |
| status-bar foreground hint (not a spec scenario) | Open a document, switch to a tool whose hint bar shows the fallback text rather than shortcut keycaps (the Eyedropper, the Ruler, the Note, the Count, the Zoom, the Hand, or the Paint Bucket), pick a non-black foreground, and read the tool hint in the status bar; then change the foreground and read it again | The fallback text carries a `· Foreground <name>` suffix naming the current foreground, and it follows a change to that colour. It is not drawn for a tool with shortcut hints (the default Move and the Brush show keycaps instead), and the bar is hidden without an open document | manual: the status-bar hint text is not part of the fill contract, and no Qt Test reads the hint's fallback string, so this is checked by eye in the PR review |

The manual row is listed in the PR body as the reviewer checklist.
