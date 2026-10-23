# Design

## Menu row

`CommandRegistry::buildMenuBar` continues to create the standard actions for
the three commands. The command table places the readout and hidden step
commands after the Window menu's Options item, so the row is the menu's last
block. `PicturaMainWindow::buildUiSizeMenu`, called from `buildMenus`, replaces
the `WindowUiSize` readout action in the Window menu with a `QWidgetAction`
whose widget stacks three rows in a `QVBoxLayout`:

- a caption row (`UI size`),
- a step row holding a minus `QToolButton`, the percentage readout label, and a
  plus `QToolButton`, and
- a reset `QToolButton` on its own row below the step row.

The buttons dispatch `WindowUiDecrease`, `WindowUiEnlarge`, and
`WindowUiReset`, so enablement, labels, shortcuts, and the shortcut editor keep
one source of truth. The three standard actions stay in the menu but hidden;
their shortcuts remain registered. The Window menu's `aboutToShow` and the live
scale setter refresh the row's readout and button enablement.

A `QWidgetAction` is deliberate: unlike a normal menu action, pressing its
buttons does not close the menu, so repeated steps need no dropdown reopening.

## Live resize

A scale change while the menu is open resizes the row's fonts and layout, but a
popup keeps the geometry it computed on show, so the row and the items after it
are clipped. `updateUiSizeMenu` therefore activates the row layout to refresh
its size hint and, when the menu is visible, resizes the menu to its size hint.
This runs on the same path as the readout and button enablement, so every scale
change (menu button, shortcut, or preference) keeps the open menu fitted.

## Shortcuts

`Ctrl+Alt++`, `Ctrl+Alt+-`, and `Ctrl+Alt+0` are free; the unmodified
`Ctrl++`, `Ctrl+-`, and `Ctrl+0` remain Zoom In, Zoom Out, and Fit On Screen.

## Window resizing

The tools column hosts a plain toolbox widget with a layout minimum height that
grows with the UI scale. In the central splitter that minimum propagates to the
main window, so at 150–200 percent the frame can exceed the screen and its
decorations fall outside the available geometry. `PanelColumn` now wraps the
tools content in a frameless, horizontally locked `QScrollArea` with
`widgetResizable` enabled. The scroll area's own minimum is small, so the
splitter and the frame can shrink; the tool grid scrolls vertically instead of
pinning the window.

The normal panel columns already sit in a scroll area and are unaffected.
