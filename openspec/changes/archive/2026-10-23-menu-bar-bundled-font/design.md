# Design

## Root cause

`PicturaMainWindow` constructs the `QMainWindow` before its constructor calls
`applyBundledUiFont`. `QApplication::setFont` installs the bundled font on the
application, but it does not replace the font of a top-level widget that already
exists. `QMenuBar` resolves its font from the main window, so the menu bar keeps
the platform default (Sans Serif 9 pt) while `QMenu` and the other chrome take
the application font (Noto Sans at the scaled pixel size). At 100 percent on a
96 DPI screen the two faces happen to measure the same number of pixels, but the
family differs; at other DPI scales the point-sized default and the pixel-sized
application font also differ in size.

## Fix

After `applyBundledUiFont`, set the window font from `QApplication::font()` in
the constructor, before `buildMenus` creates the menu bar. The menu bar then
inherits the bundled face. `setUiScalePercent` repeats the call after
`applyBundledUiFont`, so the window and the existing menu bar follow live scale
changes.

## Why not set every top-level font

Setting the font on the main window is the narrow fix. Installing the bundled
font on every top-level widget would also override any dialog that intentionally
sets its own font, and the only top-level that predates the application font is
the main window.

## Test

`tst_ui_scale::menuBarUsesBundledFont` reads the menu bar and Window menu fonts
at startup and after a scale change. It requires the bundled family and the
same pixel size as the application font, so a regression to the platform
default fails.
