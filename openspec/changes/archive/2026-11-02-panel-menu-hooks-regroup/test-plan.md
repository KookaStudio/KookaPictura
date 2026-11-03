# Test plan — panel-menu-hooks-regroup

Every scenario from `specs/ui/panel-column/spec.md`. The Coverage cell names the
Qt Test case that proves it.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `pmh_fresh` | Build a main window with a fresh session (no stored layout), so construction regroups the staged default groups into Essentials. Open the Character, Paragraph, and Actions per-widget menus. | `character.fractionalWidths` is checked (the panel's default) and disabled (no type layer); `paragraph.singleLineComposer` is checked; `actions.duplicate` is disabled (no selection). The static table alone would leave the first two unchecked and the third enabled. | `tst_workspaces::freshSessionKeepsPanelMenuHooks` |
| `pmh_workspace` | Apply the `Typography` workspace layout, which rebuilds every group and puts Character and Paragraph in a visible group. Open their per-widget menus. | The Character and Paragraph rows reflect their panel state after the rebuild, exactly as in `pmh_fresh`. | `tst_workspaces::workspaceSwitchKeepsPanelMenuHooks` |
| `pmh_move` | Show the Character panel, then move it from its group into the Layers group with `dropIntoGroupForTest`. Open the Character menu from its new group. | The Character menu still reflects the panel state (`character.fractionalWidths` checked and disabled), so the hook moved with the panel rather than staying with the old group. | `tst_workspaces::panelMoveKeepsPanelMenuHook` |

## Manual checks

None.
