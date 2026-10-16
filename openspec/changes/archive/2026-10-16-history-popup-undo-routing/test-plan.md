# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Undo from the History flyout | Create an undoable document edit, open the History panel in iconic mode, focus its list, and press Ctrl-Z. | The document edit is undone without requiring the user to close the flyout. | `tst_edit_shortcuts::ctrlZWorksWhileHistoryIconFlyoutIsOpen` |
| Text controls keep their own undo | Create an undoable document edit, type into a focused `QLineEdit`, and press Ctrl-Z. | The text edit is undone and document history is unchanged. | `tst_edit_shortcuts::textFieldKeepsNativeCtrlZUndo` |
| Native popup focus | On an interactive desktop, open the History icon flyout and press Ctrl-Z. | The document edit is undone while the flyout is open or as it closes. | manual: offscreen Qt Test cannot verify the native window manager's popup focus behavior |

## Manual rows

- Native popup focus: run the application on a desktop window system, open the History panel's iconic flyout, and press Ctrl-Z with an undoable edit available.
