# Design: move-ctrl-auto-select

`MoveToolHandler::onPress` computes the effective Auto-Select as the stored
option XOR the press's Ctrl modifier and reuses `auto_select_layer`, so the
Group/Layer dropdown still picks the target. The right-click context menu stays
on the stored option.

`ToolController::handleModifiers`, already fed by the canvas's modifier key
events, emits a new `modifiersChanged` signal. The Move options page keeps a
`moveCtrlFlip_` flag and, under a signal blocker, shows the stored option XOR
the flag; its `toggled` handler stores `checked XOR flip`, so a click on the
box while Ctrl is held still toggles the real option.

Known limit: a Ctrl release while the window is unfocused is not seen, so the
box stays flipped until the next Ctrl key event. Presses read their own
modifiers and are unaffected.
