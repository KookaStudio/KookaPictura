# Design

## Context

`PicturaRawDialog` owns one `AcrSettings` map. Each tab panel is built with a
pointer to that map and writes a control's key when it moves. The Presets panel
is a `QListWidget` whose rows were never wired. The tab strip is a plain
`QTabBar` with icon-only tabs, so the theme's global `QTabBar::tab` rules and the
style's base line draw through it.

The settings text already round-trips through `pictura_codec::pictura_raw_to_text`
and `pictura_raw_from_text`. The same module owns `pictura_raw_to_xmp` and
`pictura_raw_from_xmp`; the dialog only needs those two functions exposed to C++.

## Decisions

**One settings replacement path.** Every fx action replaces the whole settings
map and then rebuilds the eight control panels. Rebuilding is simpler than
teaching each panel a reload path: the panels read their values once in their
constructors and the tone-curve graph needs the preview image again. The Presets
panel is not rebuilt, so the widget that emitted the action survives the slot.
The old control panels are deleted synchronously (they are not the emitter) and
new ones are inserted at indices 0–7, leaving the Presets panel at index 8.

**Initial settings are kept as text.** `Image Settings` restores the exact text
the dialog was opened with, so unknown keys survive a round trip through the
dialog. `Clear Imported Settings` uses the same value; it is enabled only after a
settings file has been loaded.

**Previous Conversion is session state.** The app opens one document at a time,
so a process-wide value is enough: `accept()` records the encoded settings, and a
later dialog can restore them. A test reset hook clears it between tests.

**Load/Save are split from the file dialog.** `loadSettingsFile(path)` and
`saveSettingsFile(path)` hold the behavior; the menu actions only choose a path
with `QFileDialog`. Tests call the path methods, so no native dialog is involved.

**XMP stays in Rust.** A small bridge module exposes
`pictura_raw_settings_from_xmp(QByteArray) -> QString` and
`pictura_raw_settings_to_xmp(QString) -> QByteArray`. The dialog treats the
packet as opaque bytes. A loaded file must contain `crs:` or it is refused.

**Disabled preset entries stay visible.** Custom Settings, Preset Settings, and
Apply Preset keep their rows so the list matches Camera Raw, but their items are
not enabled until the preset library exists (#633).

**The tab strip is a clean icon row.** `setDrawBase(false)` removes the dotted
base line, `setExpanding(true)` gives every tab the same share of the width, and
a fixed `QSize(16, 16)` icon size aligns the glyphs. An `acrTabBar` object name
lets the theme drop the tab borders and fill only the selected tab. The outer
layout gets an explicit margin and spacing so the preview no longer touches the
controls column.

**Wheel zoom reuses the ladder.** `wheelEvent` maps a positive `angleDelta().y()`
to `stepZoom(1)` and a negative one to `stepZoom(-1)`, exactly as the + and −
buttons do. No new zoom model is introduced.

## Risks

- Rebuilding panels while the Presets list owns the click requires the Presets
  panel to stay out of the rebuild. Keeping it at index 8 and rebuilding only
  0–7 is the guard.
- `QStackedWidget::removeWidget` does not reparent, so the old panels are
  deleted immediately to avoid duplicate `findChild` hits in tests.
- The XMP writer keeps every unknown property; loading then saving a file the
  dialog did not author preserves its other bytes only when the dialog saves back
  over the same packet. `Save Settings…` writes a fresh packet, which is the
  documented behavior for a new file.
