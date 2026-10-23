## ADDED Requirements

### Requirement: Menu bar uses the pinned application font

The main window and its menu bar SHALL use the pinned application font at the
current UI scale. The menu bar's resolved font family SHALL be the bundled
family and its pixel size SHALL equal the application font's pixel size at
startup and after every live UI-scale change. Menu popups SHALL use the same
family and pixel size as the menu bar.

#### Scenario: The menu bar uses the bundled font [bfn_menu_bar_font]

- **WHEN** the application starts at 100 percent UI scale
- **THEN** `menuBar()->font()` resolves to `Noto Sans` at 12 pixels and matches
  the Window menu's font

#### Scenario: The menu bar follows the UI scale [bfn_menu_bar_scale]

- **WHEN** the UI scale changes to 150 percent
- **THEN** the menu bar and the Window menu both resolve to `Noto Sans` at 18
  pixels
