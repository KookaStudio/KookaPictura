# Spec Delta

## MODIFIED Requirements

### Requirement: Pinned application UI font

The system SHALL register the bundled faces after the `QApplication` exists and
SHALL install Noto Sans as the application UI font on every OS at the theme's
current UI scale times the pinned 12-pixel base. At the default 100 percent
scale the application font SHALL be 12 pixels; at other scales it SHALL be
`Theme::scaled(12)`. The chrome size derivations that run two pixels under the
application font SHALL read the current application pixel size rather than a
point size.

#### Scenario: The application font is the bundled family [bfn_app_font]

- **WHEN** the application starts at 100 percent UI scale
- **THEN** `QApplication::font().family()` is `Noto Sans` and
  `QApplication::font().pixelSize()` is `12`

#### Scenario: The font follows the UI scale [bfn_font_scale]

- **WHEN** the UI scale is 150 percent
- **THEN** `QApplication::font().pixelSize()` is `18`

#### Scenario: The chrome keeps its relative size [bfn_chrome_size]

- **WHEN** a tab label or the status footer derives its size from the
  application font
- **THEN** it is set two pixels under the current application pixel size
