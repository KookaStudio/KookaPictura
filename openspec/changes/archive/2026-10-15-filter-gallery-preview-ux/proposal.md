# Proposal

## Why

The Filter Gallery's preview (capability `imaging/filter-gallery`) shipped with
a bare zoom percentage label and +/- buttons, a text-glyph thumbnails toggle,
and no wheel zoom. CS6 offers a zoom drop-down of preset levels and the three
named views, a wheel that zooms, and an icon toggle whose appearance matches the
other buttons. Separately, a parameter slider in the shared filter controls does
not focus its value box, so a click on the slider leaves the arrow keys adjusting
nothing.

## What Changes

- **Zoom drop-down.** The gallery's zoom level becomes an editable drop-down
  showing the live percentage and listing CS6's presets — 6 %, 12 %, 25 %, 33 %,
  50 %, 66 %, separator, 100 %, separator, 200 %…1600 %, separator, Actual Pixels,
  Fit in View, Fit on Screen.
- **Wheel zoom.** A mouse wheel over the preview zooms one level.
- **Thumbnails toggle.** The toggle uses the Lucide `panel-right-close` /
  `panel-right-open` icon and the same flat button surface as the other icon
  buttons, instead of the «/» text glyph.
- **Slider activates its value box.** Clicking a parameter slider focuses the
  value box beside it, so Up/Down (and typing) adjust the same parameter.

Not breaking: the committed result, the panned/zoomed preview pixels, and the
`Ctrl`+`+` / `Ctrl`+`-` zoom shortcuts are unchanged. A larger preview pan
performance improvement is tracked separately (issue #452).

## Capabilities

### New Capabilities

<!-- None: this change extends existing capabilities. -->

### Modified Capabilities

- `imaging/filter-gallery`: new requirements for the zoom control and the
  thumbnails toggle.
- `imaging/filter-app-ui`: a new requirement for the parameter slider focusing
  its value box.

## Impact

- `crates/pictura-app/cpp/filter_gallery_dialog.{h,cpp}` — the zoom drop-down,
  wheel handling, and the toggle icon.
- `crates/pictura-app/cpp/filter_param_controls.cpp` — slider press focuses the
  spin box.
- `assets/icons/panel.rightClose.svg`, `panel.rightOpen.svg` (new) +
  `assets/icons/lucide-map.json` + `assets/pictura.qrc`.
- `crates/pictura-app/cpp/tests/tst_filter_gallery.cpp` — zoom drop-down, wheel,
  and slider-focus checks.
- No new dependency, no engine-crate or cxx-qt bridge change.
