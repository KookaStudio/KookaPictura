# Design

## Footer font ownership

The footer size is `QApplication::font().pixelSize() - 2`, matching the tab
chrome. Two things kept it from taking effect:

1. An application stylesheet resolves a child widget's font against the
   application font instead of an ancestor's explicit font, so setting it on the
   `QStatusBar` alone left the labels at 12 px.
2. `applyStatusBarScale()` only resized the bar and its items; the font was
   computed once in `buildStatusBar()`, at the scale the frame was built with.

`applyStatusBarScale()` therefore sets the font on the bar *and* on every text
carrier (zoom field, size label, options button, backend label, action label,
tool hint strip, search hint) in the same loop that applies the item heights. It
reads the live application font each time, so construction, a live scale change,
and a session restore share one computation. The item heights stay
`Theme::scaled(22)`; a 10 px footer needs 14 px of line height and a 7 px footer
9 px, both below the 22 px and 17 px items, so no height floor is needed.

## Scale application order

`applyStatusBarScale()` applies the font before resizing the bar and the items,
so the bar's own `font()` already reflects the new scale when the sizes are set.
`setUiScalePercent()` calls `applyBundledUiFont()` (which changes the application
font) before `applyUiScaleToChrome()`, which reaches `applyStatusBarScale()`, so
the helper sees the new application font. The session-restore path resolves the
scale in the constructor before `buildStatusBar()` runs, so a frame built at
75 % starts with a 7 px footer.

## Test seam

`tst_ui_scale::statusBarTextFollowsScale` reads the bar and the readout widgets
through the object names the frame already exposes (`statusSizeLabel`,
`statusBackendLabel`, `toolHintBar`) and compares each font against the
application font minus two pixels, plus the readout height against
`QFontMetrics::height()`. It repeats after `setUiScalePercent(75)`.
