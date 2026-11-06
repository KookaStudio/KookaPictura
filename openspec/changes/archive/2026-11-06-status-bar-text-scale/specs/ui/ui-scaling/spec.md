## MODIFIED Requirements

### Requirement: Live scale application

A change to the UI scale SHALL apply immediately without restarting the
application. The application font, the stylesheet's pixel metrics, tab fonts,
tool-slot geometry, status-bar chrome and footer text, menu-corner icons,
panel-column bounds, and panel-group icon controls SHALL follow the new scale.
The status bar's footer text SHALL be recomputed from the new application font
and SHALL be set on the bar and on each of its readouts, hint strip, and search
hint, so it is two pixels under the application font and fits its item at every
scale. The document image, its zoom, and its rendered pixels SHALL NOT change.
The chosen percentage SHALL persist in the session and be restored before the
chrome is built.

#### Scenario: Font and stylesheet follow the scale [uis_live_font_qss]

- **WHEN** the scale changes from 100 to 200 percent
- **THEN** the application font pixel size doubles and stylesheet lengths such
  as the menu-bar padding double

#### Scenario: Shell chrome and panels follow the scale [uis_live_chrome]

- **WHEN** the scale changes while the frame is open
- **THEN** the status bar, tool slots, menu-corner icons, and panel-column
  width bounds use the scaled metrics

#### Scenario: The status-bar footer follows the scale [uis_status_bar_font]

- **WHEN** the scale changes to 75 percent
- **THEN** the status bar and its readouts, hint strip, and search hint use a
  font two pixels under the 75-percent application font, and every readout is
  tall enough for that font's metrics, so the text is not clipped

#### Scenario: Canvas pixels are unaffected [uis_canvas_unaffected]

- **WHEN** the scale changes with a document open
- **THEN** the canvas's zoom and displayed document pixels are unchanged

#### Scenario: The scale survives a restart [uis_persist]

- **WHEN** the scale is set and the session is reloaded
- **THEN** the restored percentage is the stored percentage
