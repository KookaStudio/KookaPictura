# Design

## Scale model

`Theme` owns one process-wide UI scale. It is an integer percentage chosen from
`{75, 80, 90, 100, 110, …, 200}` so the step is 10 % except for the single
5-point step from 75 to 80 needed to include the owner's 75 % floor.
`Theme::uiScalePercent()` reads it, `Theme::setUiScalePercent()` normalizes an
arbitrary value to the nearest choice, and `Theme::scaled(px)` rounds
`px * percent / 100` with a one-pixel floor. The default is 100, which leaves
every existing metric and golden unchanged.

The percent is global. Qt's device-pixel-ratio scaling still runs underneath,
so this is a multiplier on logical pixels, not a replacement for per-monitor
DPI handling.

## Stylesheet and font

`styleSheetFor()` scales literal `Npx` lengths in the QSS template before token
substitution. Token metrics are scaled in `Theme::tokens()` except `shade-step`,
which is a colour operation, not a length. The `font-size-tab` and
`font-size-input` placeholders keep deriving from the live application font, so
they are not scaled twice.

`bundledUiFont()` uses `Theme::scaled(kBundledUiFontPx)`. `applyTabBarFont()`
and the status footer continue to derive from the live application pixel size
and stay two pixels under it. This is the only point where the fixed-12-pixel
contract in `ui/bundled-fonts` changes: 12 px remains the 100 % anchor.

## Live application

`PicturaMainWindow` stores `uiScalePercent_`, restored from the session before
the menu bar, panels, tools, and status bar are built. `setUiScalePercent()`
then, in order:

1. normalizes and stores the value, calls `Theme::setUiScalePercent()`;
2. rebuilds the application font and the document tab font;
3. re-applies the theme, which rebuilds the QSS and palette;
4. re-scales the tool slots, status bar, menu-corner icons, panel columns, and
   panel groups;
5. refreshes the command registry and persists the session.

Panel columns keep their absolute widths in the session but scale them by the
old/new ratio on a live change, and their bounds (`300…400`) scale with the
theme. An iconic column scales its remembered normal width instead.

The document canvas is untouched: the picture view, zoom, and rendered pixels
do not read the UI scale.

## Commands and Preferences

The Window menu gains a disabled `UI Size: N%` readout and three commands:
Enlarge, Decrease, Reset. Enlarge/Decrease move through the choice list and are
disabled at the ends; Reset is disabled at 100 %. The Interface preferences
page gains a combo listing the same choices. Both read and write the frame's
single setter, so they stay in step in either direction.

## Alternatives rejected

- Four CS6 sizes with a restart: coarser than requested, and the owner asked
  for live application.
- Scaling only the application font: icons and panel widths would not grow, so
  the chrome would reflow badly on a HiDPI display.
- A per-widget style override at every call site: duplicates the scale and
  drifts. The theme is the single source.
