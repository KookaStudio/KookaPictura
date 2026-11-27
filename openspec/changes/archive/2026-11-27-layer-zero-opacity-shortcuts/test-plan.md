# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `zero_opacity_canvas` | Select two layers with distinct opacity values and tap 0 twice on the Move canvas. Undo. | Both values reach zero without changing Fill. One Undo restores both originals. | `tst_layer_zero_shortcuts::canvasOpacity` |
| `zero_fill_tree` | Select a layer and send two synthetic Shift+`Qt::Key_0` presses to the Layers tree. Undo. | Fill reaches zero in one step. Opacity retains its value. | `tst_layer_zero_shortcuts::treeFill` |
| `zero_eligibility` | Try both sequences on Background and fully locked layers. Try Fill, then Opacity twice on a group. | Only the first group opacity gesture adds a history state. | `tst_layer_zero_shortcuts::eligibilityAndNoOp` |
| `zero_ignore` | Send a lone zero, auto-repeat, slow pair, mixed Shift, an intervening key and presses split between the canvas and tree. | Each incomplete sequence preserves property values and history. | `tst_layer_zero_shortcuts::interruptedSequence` |
| `zero_context` | Send one zero, change focus, document or selection, and send another; repeat each after restoring the original context. | No pair changes a layer property or history. | `tst_layer_zero_shortcuts::contextChanges` |
| `zero_modifiers` | Send Ctrl, Alt and Meta zero combinations, including Ctrl+0 and Ctrl+Alt+0, alone and between plain zeros. | No layer gesture fires; registered Ctrl+0 and Ctrl+Alt+0 commands still act. | `tst_layer_zero_shortcuts::modifiedKeys` |
| `zero_transform` | Begin a pending transform and send both gestures on the Move canvas and Layers tree; use a transform key afterwards. | Layer properties and history stay unchanged by the gestures, and the transform key works. | `tst_layer_zero_shortcuts::pendingTransform` |
| `zero_editor` | Enter 00 into the Layers opacity field without committing. | The editor receives the digits without a shortcut history entry. | `tst_layer_zero_shortcuts::editorInput` |
| Physical Shift+0 and keypad | On US and Finnish layouts, select Move and press Shift+0 twice; Undo, try Shift+9 twice, then keypad zero twice. | Shift+0 sets Fill to zero on each layout; Shift+9 does not trigger it; keypad zero sets Opacity to zero. | manual: Synthetic Qt events cannot reproduce the operating system's keyboard layout and modifier translation. |
